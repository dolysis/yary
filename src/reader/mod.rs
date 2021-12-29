/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

mod error;

pub use error::{ReaderError, ReaderResult};
use private::Sealed;

use crate::{
    queue::Queue,
    reader::error::ReaderResult as Result,
    scanner::{entry::TokenEntry, flag::Flags, Scanner},
};

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
    fn drive<'de>(
        &'de self,
        scanner: &mut Scanner,
        queue: &mut Queue<TokenEntry<'de>>,
        options: Flags,
    ) -> Result<()>;

    /// Hint to the underlying implementation that no live
    /// references exist to any data read below
    /// the given .bound, and that it may unload the given
    /// memory.
    ///
    /// SAFETY:
    ///     It is only safe to call this function after the
    ///     caller has ensured there cannot be any live
    ///     references to content below the provided .bound.
    unsafe fn consume(&self, _bound: usize) -> Result<()>
    {
        Ok(())
    }
}

/// Responsible for driving a Read implementation,
/// tokenizing the byte stream in preparation for
/// for an Event stream
#[derive(Debug)]
pub struct Reader<'de, T: 'de>
{
    scanner: Scanner,
    queue:   Queue<TokenEntry<'de>>,

    options:   Flags,
    exhausted: bool,

    inner: &'de T,
}

impl<'de, T> Reader<'de, T>
where
    T: Read,
{
    pub fn new(read: &'de T, opts: Flags) -> Self
    {
        Self {
            scanner:   Scanner::new(),
            queue:     Queue::new(),
            options:   opts,
            exhausted: false,
            inner:     read,
        }
    }

    pub fn scan_tokens(&mut self) -> Result<&mut Queue<TokenEntry<'de>>>
    {
        let start = self.queue.len();

        self.inner
            .drive(&mut self.scanner, &mut self.queue, self.options)?;

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
        options: Flags,
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
pub struct PeekReader<'de, T: 'de>
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

    pub fn pop(&mut self) -> Result<Option<TokenEntry<'de>>>
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

    pub fn peek(&mut self) -> Result<Option<&mut TokenEntry<'de>>>
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

    fn take_next(&mut self) -> Result<()>
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
    pub trait Sealed
    {
    }
}
