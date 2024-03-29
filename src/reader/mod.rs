/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! The module contains the adapters for converting plain
//! bytes to a representation that is useful for parsing.
//!
//! This behavior is defined by the [`Read`] trait, which is
//! sealed, and cannot be implemented outside of this
//! library.
//!
//! There are three conversions that are supported, each
//! with a top level function that handles the conversion:
//!
//! - `&str` => [`from_utf8()`]
//! - `&[u8]` => [`try_from_bytes()`]
//! - `T: std::io::Read` => [`from_read()`]

pub mod borrow;
pub mod owned;

pub(crate) mod error;

use crate::{
    error::Error,
    queue::Queue,
    reader::{
        borrow::BorrowReader,
        error::{ReadError, ReaderResult},
        owned::OwnedReader,
        private::Sealed,
    },
    scanner::{entry::TokenEntry, flag::Flags as ScannerFlags, Scanner},
};

/// Instantiate a new [`Read`]er from the given UTF8 string
/// slice
///
/// ## Examples
///
/// ```rust
/// use yary::reader::from_utf8;
///
/// let yaml = "{a yaml: mapping}";
///
/// let reader = from_utf8(yaml);
/// ```
pub fn from_utf8(utf8: &str) -> BorrowReader<'_>
{
    BorrowReader::new(utf8)
}

/// Instantiate a new [`Read`]er from the given
/// [`std::io::Read`] source.
///
/// ## Examples
///
/// ```no_run
/// use std::fs::File;
///
/// use yary::reader::from_read;
///
/// let file = File::open("config.yaml")?;
///
/// let reader = from_read(file);
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn from_read<R>(src: R) -> OwnedReader
where
    R: std::io::Read + 'static,
{
    OwnedReader::new(src)
}

/// Try instantiate a new [`Read`]er from the given byte
/// slice.
///
/// ## Errors
///
/// This function will error if the provided byte slice is
/// not valid UTF8
///
/// ## Examples
///
/// ```rust
/// use yary::reader::try_from_bytes;
///
/// let yaml = b"[some, valid, yaml]";
///
/// let reader = try_from_bytes(yaml);
/// assert!(reader.is_ok())
/// ```
pub fn try_from_bytes(slice: &[u8]) -> std::result::Result<BorrowReader<'_>, Error>
{
    BorrowReader::try_from_bytes(slice).map_err(Into::into)
}

/// Sealed interface over the functionality that
/// transforms a byte stream into [Token][crate::token::
/// Token]s.
///
/// Note the key feature here is `&'de self`. Namely, an
/// immutable reference through which any internal mutation
/// must not be visible
pub trait Read: std::fmt::Debug + Sealed
{
    /// Drive the .scanner from the byte stream with the
    /// provided .options, placing output into the
    /// .queue
    fn drive<'de>(&'de self, cxt: ReadContext<'_, '_, 'de>) -> Result<(), ReadError>;

    /// Hint to the underlying implementation that no live
    /// references exist to any data read below
    /// the given .bound, and that it may unload the given
    /// memory.
    ///
    /// ## Safety
    ///
    /// It is only safe to call this function after the
    /// caller has ensured there cannot be any live
    /// references to content below the provided .bound.
    unsafe fn consume(&self, _bound: usize) -> Result<(), ReadError>
    {
        Ok(())
    }
}

/// An intentionally opaque type which hides the
/// implementation details of [`Read`] methods.
pub struct ReadContext<'a, 'b, 'de>
{
    scanner: &'a mut Scanner,
    queue:   &'b mut Queue<TokenEntry<'de>>,
    flags:   ScannerFlags,
}

impl<'a, 'b, 'de> ReadContext<'a, 'b, 'de>
{
    fn new(
        scanner: &'a mut Scanner,
        queue: &'b mut Queue<TokenEntry<'de>>,
        flags: ScannerFlags,
    ) -> Self
    {
        Self {
            scanner,
            queue,
            flags,
        }
    }
}

/// Responsible for driving a Read implementation,
/// tokenizing the byte stream in preparation for
/// for an Event stream
#[derive(Debug)]
pub(crate) struct Reader<'de, T: 'de>
{
    scanner: Scanner,
    queue:   Queue<TokenEntry<'de>>,

    options:   ScannerFlags,
    exhausted: bool,

    inner: &'de T,
}

impl<'de, T> Reader<'de, T>
where
    T: Read,
{
    pub fn new(read: &'de T, opts: ScannerFlags) -> Self
    {
        Self {
            scanner:   Scanner::new(),
            queue:     Queue::new(),
            options:   opts,
            exhausted: false,
            inner:     read,
        }
    }

    pub fn scan_tokens(&mut self) -> ReaderResult<&mut Queue<TokenEntry<'de>>>
    {
        let start = self.queue.len();

        self.inner.drive(ReadContext::new(
            &mut self.scanner,
            &mut self.queue,
            self.options,
        ))?;

        self.exhausted = start == self.queue.len();

        Ok(&mut self.queue)
    }

    pub fn is_exhausted(&self) -> bool
    {
        self.exhausted && self.queue.is_empty()
    }

    pub fn queue_mut(&mut self) -> &mut Queue<TokenEntry<'de>>
    {
        &mut self.queue
    }

    pub(crate) fn queue(&self) -> &Queue<TokenEntry<'de>>
    {
        &self.queue
    }

    pub(crate) fn from_parts(
        read: &'de T,
        options: ScannerFlags,
        queue: Queue<TokenEntry<'de>>,
        exhausted: bool,
    ) -> Self
    {
        Self {
            scanner: Scanner::new(),
            queue,
            options,
            exhausted,
            inner: read,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PeekReader<'de, T: 'de>
{
    peek:   Option<TokenEntry<'de>>,
    reader: Reader<'de, T>,
}

impl<'de, T> PeekReader<'de, T>
where
    T: Read,
{
    pub fn new(reader: Reader<'de, T>) -> Self
    {
        Self { peek: None, reader }
    }

    pub fn pop(&mut self) -> ReaderResult<Option<TokenEntry<'de>>>
    {
        match self.peek.take()
        {
            Some(entry) => Ok(Some(entry)),
            None if !self.reader.is_exhausted() =>
            {
                self.take_next()?;

                Ok(self.peek.take())
            },
            None => Ok(None),
        }
    }

    pub fn peek(&mut self) -> ReaderResult<Option<&mut TokenEntry<'de>>>
    {
        match self.peek
        {
            Some(ref mut entry) => Ok(Some(entry)),
            None if !self.reader.is_exhausted() =>
            {
                self.take_next()?;

                Ok(self.peek.as_mut())
            },
            None => Ok(None),
        }
    }

    pub fn consume(&mut self) -> bool
    {
        self.peek.take().is_some()
    }

    pub fn into_inner(self) -> (Reader<'de, T>, Option<TokenEntry<'de>>)
    {
        let Self { peek, reader } = self;

        (reader, peek)
    }

    pub(crate) fn queue(&self) -> &Queue<TokenEntry<'de>>
    {
        self.reader.queue()
    }

    fn take_next(&mut self) -> ReaderResult<()>
    {
        // Ensure we don't overwrite an existing entry
        if self.peek.is_some()
        {
            return Ok(());
        }

        // If the queue is empty, make an attempt to retrieve more
        // tokens from the Reader
        if self.reader.queue_mut().is_empty()
        {
            self.reader.scan_tokens()?;
        }

        self.peek = self.reader.queue_mut().pop();

        Ok(())
    }
}

mod private
{
    pub trait Sealed {}
}

#[cfg(test)]
mod test_util
{
    use super::*;
    use crate::{
        scanner::flag::O_ZEROED,
        token::Marker::{self, *},
    };

    pub(super) type TestResult = anyhow::Result<()>;

    pub(super) const TEST_FLAGS: ScannerFlags = O_ZEROED;
    pub(super) const TEST_DATA: [&str; 3] = [YAML_SCALAR, YAML_SEQUENCE, YAML_MAPPING];

    pub(super) const YAML_SCALAR: &str = "'a simple, root scalar'";
    pub(super) const YAML_SEQUENCE: &str = "
        - A
        - YAML
        - Sequence";
    pub(super) const YAML_MAPPING: &str = "{'A YAML': Mapping}";

    pub(super) const SCALAR_MARKERS: [Marker; 3] = [StreamStart, Scalar, StreamEnd];
    pub(super) const SEQUENCE_MARKERS: [Marker; 10] = [
        StreamStart,
        BlockSequenceStart,
        BlockEntry,
        Scalar,
        BlockEntry,
        Scalar,
        BlockEntry,
        Scalar,
        BlockEnd,
        StreamEnd,
    ];
    pub(super) const MAPPING_MARKERS: [Marker; 8] = [
        StreamStart,
        FlowMappingStart,
        Key,
        Scalar,
        Value,
        Scalar,
        FlowMappingEnd,
        StreamEnd,
    ];

    fn data_into_read<T, F>(data: &'static str, f: F) -> T
    where
        F: Fn(&'static str) -> T,
        T: Read,
    {
        f(data)
    }

    pub(super) fn t_scalar<T, F>(f: F) -> T
    where
        F: Fn(&'static str) -> T,
        T: Read,
    {
        data_into_read(YAML_SCALAR, f)
    }

    pub(super) fn t_sequence<T, F>(f: F) -> T
    where
        F: Fn(&'static str) -> T,
        T: Read,
    {
        data_into_read(YAML_SEQUENCE, f)
    }

    pub(super) fn t_mapping<T, F>(f: F) -> T
    where
        F: Fn(&'static str) -> T,
        T: Read,
    {
        data_into_read(YAML_MAPPING, f)
    }

    pub(super) fn drive_test<'a, T>(r: &mut Reader<'a, T>, expected: &[Marker]) -> TestResult
    where
        T: Read,
    {
        use pretty_assertions::assert_eq;

        let mut tokens;
        let mut expected = expected.iter().copied();

        while !r.is_exhausted()
        {
            tokens = r.scan_tokens()?;

            while let Some(actual) = tokens.pop().map(|entry| entry.marker())
            {
                assert_eq!(expected.next(), Some(actual));
            }
        }

        Ok(())
    }

    macro_rules! test_reader {
        ($read_fn:expr) => {
            test_reader!{@inner $read_fn}
        };
        (@inner $read_fn:expr) => {
            use $crate::reader::test_util as __util;

            test_reader![@test
                simple_scalar => { __util::t_scalar, $read_fn, __util::SCALAR_MARKERS },
                simple_sequence => { __util::t_sequence, $read_fn, __util::SEQUENCE_MARKERS },
                simple_mapping => { __util::t_mapping, $read_fn, __util::MAPPING_MARKERS }
            ];
        };
        (@test $( $t_name:ident => { $data_fn:path, $read_fn:expr, $expected:expr }),+ ) => {
            $(
                #[test]
                fn $t_name() -> __util::TestResult
                {
                    let src = $data_fn($read_fn);
                    let mut reader = $crate::reader::Reader::new(&src, __util::TEST_FLAGS);

                    __util::drive_test(&mut reader, &$expected)
                }
            )+
        };
    }

    pub(super) use test_reader;
}
