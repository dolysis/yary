/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Contains an implementation of [`Read`](super::Read) for
//! borrowed UTF8 slices (`&str`s).

use super::{
    error::{ReadError, ReaderResult},
    private, Read, ReadContext, Reader,
};
use crate::scanner::flag::{Flags, O_EXTENDABLE};

/// A [`Read`](super::Read) implementor for borrows.
#[derive(Debug, Clone)]
pub struct BorrowReader<'de>
{
    data: &'de str,
}

impl<'de> BorrowReader<'de>
{
    /// Instantiate a new [`BorrowReader`] from the given
    /// UTF8 slice
    pub fn new(data: &'de str) -> Self
    {
        Self { data }
    }

    /// Instantiate a new [`BorrowReader`] from the given
    /// bytes, returning an error if they are not
    /// valid UTF8.
    ///
    /// This is a small wrapper around [`BorrowReader::new`]
    /// which attempts to assert that the underlying
    /// byte slice is UTF8.
    pub(crate) fn try_from_bytes(data: &'de [u8]) -> ReaderResult<Self>
    {
        let this = std::str::from_utf8(data).map(Self::new)?;

        Ok(this)
    }

    pub(crate) fn new_reader(&'de self, opts: Flags) -> Reader<'de, Self>
    {
        Reader::new(self, opts)
    }
}

impl<'a> Read for BorrowReader<'a>
{
    fn drive<'de>(&'de self, cxt: ReadContext<'_, '_, 'de>) -> Result<(), ReadError>
    {
        // This implementation is never extendable, so we remove the
        // option from the set if it exists
        cxt.scanner
            .scan_tokens(cxt.flags & !O_EXTENDABLE, self.data, cxt.queue)?;

        Ok(())
    }

    unsafe fn consume(&self, _bound: usize) -> Result<(), ReadError>
    {
        Ok(())
    }
}

impl private::Sealed for BorrowReader<'_> {}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::reader::test_util::test_reader;

    test_reader! {BorrowReader::new}
}
