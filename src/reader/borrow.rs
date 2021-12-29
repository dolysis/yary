/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use super::{error::ReaderResult as Result, private, Read, Reader};
use crate::{
    queue::Queue,
    scanner::{
        entry::TokenEntry,
        flag::{Flags, O_EXTENDABLE},
        Scanner,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct BorrowReader<'de>
{
    data: &'de str,
}

impl<'de> BorrowReader<'de>
{
    pub fn new(data: &'de str) -> Self
    {
        Self { data }
    }

    pub fn try_from_bytes(data: &'de [u8]) -> Result<Self>
    {
        let this = std::str::from_utf8(data).map(Self::new)?;

        Ok(this)
    }

    pub fn new_reader(&'de self, opts: Flags) -> Reader<'de, Self>
    {
        Reader::new(self, opts)
    }
}

impl<'a> Read for BorrowReader<'a>
{
    fn drive<'de>(
        &'de self,
        scanner: &mut Scanner,
        queue: &mut Queue<TokenEntry<'de>>,
        options: Flags,
    ) -> Result<()>
    {
        // This implementation is never extendable, so we remove the
        // option from the set if it exists
        scanner.scan_tokens(options & !O_EXTENDABLE, self.data, queue)?;

        Ok(())
    }

    unsafe fn consume(&self, _bound: usize) -> Result<()>
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
