/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{cell::UnsafeCell, fmt, io};

use super::{
    error::{ReadError, ReaderResult},
    private::Sealed,
    Read, ReadContext, Reader,
};
use crate::{
    queue::Queue,
    scanner::{
        entry::TokenEntry,
        error::ScanError,
        flag::{Flags, O_EXTENDABLE},
        Scanner,
    },
};

const DEFAULT_BUFFER_SIZE: usize = 8 * 1024;

#[derive(Debug)]
pub struct OwnedReader
{
    inner: ReadHolder,
}

impl OwnedReader
{
    pub fn new<T>(src: T) -> Self
    where
        T: io::Read + 'static,
    {
        let inner = ReadHolder::new(src);

        Self { inner }
    }

    pub(crate) fn new_reader(&self, opts: Flags) -> Reader<'_, Self>
    {
        Reader::new(self, opts)
    }

    fn drive_scanner<'de>(
        &'de self,
        scanner: &mut Scanner,
        queue: &mut Queue<TokenEntry<'de>>,
        mut opts: Flags,
    ) -> ReaderResult<()>
    {
        loop
        {
            match self.inner.is_exhausted()
            {
                true => opts.remove(O_EXTENDABLE),
                false => opts.insert(O_EXTENDABLE),
            }

            match scanner.scan_tokens(opts, self.inner.data(), queue)
            {
                Err(ScanError::Extend) =>
                {
                    let read_to = scanner.offset();

                    self.inner.read_next_chunk(Some(read_to))?;

                    scanner.reset_offset();
                },

                Ok(_) => return Ok(()),
                Err(e) => return Err(e.into()),
            }
        }
    }
}

impl Read for OwnedReader
{
    fn drive<'de>(&'de self, cxt: ReadContext<'_, '_, 'de>) -> Result<(), ReadError>
    {
        self.drive_scanner(cxt.scanner, cxt.queue, cxt.flags)
            .map_err(Into::into)
    }

    unsafe fn consume(&self, _bound: usize) -> Result<(), ReadError>
    {
        Ok(())
    }
}

impl Sealed for OwnedReader {}

#[derive(Debug)]
struct ReadHolder
{
    inner: UnsafeCell<Impl>,
}

impl ReadHolder
{
    pub fn new<T>(src: T) -> Self
    where
        T: io::Read + 'static,
    {
        let inner = Impl::new(src).into();

        Self { inner }
    }

    pub fn read_next_chunk(&self, read_to: Option<usize>) -> ReaderResult<()>
    {
        let inner: &mut Impl = unsafe { &mut *self.inner.get() };

        inner.refresh_buffer(read_to)
    }

    pub fn data(&self) -> &str
    {
        // SAFETY:
        //
        // We never drop the contents being referenced here.
        //
        // The whole point of this structure is to ensure we never
        // reallocate *any* of Impl's contents, only moving
        // the (cap,len,ptr) triple that makes up Vecs (and
        // consequently Strings)
        //
        // This section REQUIRES the following invariants:
        //
        //  1. Impl's .head and .tail(s) never perform any operation
        //     that could invalidate references
        //     (realloc of any kind)
        //  2. Impl must not drop any of the allocated data before
        //     ReadHolder (ourselves) is dropped
        let inner: &Impl = unsafe { &*self.inner.get() };

        inner.data()
    }

    fn is_exhausted(&self) -> bool
    {
        let inner: &Impl = unsafe { &*self.inner.get() };

        inner.exhausted
    }
}

struct Impl
{
    head: String,
    tail: Vec<String>,

    source:    Box<dyn io::Read + 'static>,
    exhausted: bool,
}

impl Impl
{
    pub fn new<T>(src: T) -> Self
    where
        T: io::Read + 'static,
    {
        let source = Box::new(src);

        Self {
            head: String::new(),
            tail: Vec::new(),

            source,
            exhausted: false,
        }
    }

    pub fn data(&self) -> &str
    {
        &self.head
    }

    fn refresh_buffer(&mut self, copy_from: Option<usize>) -> ReaderResult<()>
    {
        // Calculate next allocation chunk
        let cap = (DEFAULT_BUFFER_SIZE * usize::max(self.tail.len(), 1) + copy_from.unwrap_or(0))
            .next_power_of_two();
        let mut new = Vec::new();

        // Copy any data that is marked as unread into the next
        // buffer
        if let Some(mark) = copy_from
        {
            new.extend_from_slice(&self.head.as_bytes()[mark..]);
        }

        // Fill the new buffer, checking if .src has been exhausted
        self.exhausted = read_fill(Take::new(&mut self.source, cap), &mut new)?;

        // Validate buffer is UTF8
        let new = String::from_utf8(new).map_err(|e| e.utf8_error())?;

        // Swap the new and old heads, pushing the old head into the
        // held tails
        let old = std::mem::replace(&mut self.head, new);
        self.tail.push(old);

        Ok(())
    }
}

impl fmt::Debug for Impl
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("Impl")
            .field("head", &self.head)
            .field("tail", &self.tail)
            .field("source", &"dyn <std::io::Read>")
            .field("exhausted", &self.exhausted)
            .finish()
    }
}

fn read_fill<T>(mut src: T, buf: &mut Vec<u8>) -> io::Result<bool>
where
    T: io::Read,
{
    let amt = src.read_to_end(buf)?;

    Ok(amt == 0)
}

struct Take<'a>
{
    limit: usize,
    inner: &'a mut dyn io::Read,
}

impl<'a> Take<'a>
{
    fn new(read: &'a mut dyn io::Read, limit: usize) -> Self
    {
        Self { inner: read, limit }
    }
}

impl<'a> io::Read for Take<'a>
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        // Don't call into inner reader at all at EOF because it may
        // still block
        if self.limit == 0
        {
            return Ok(0);
        }

        let max = usize::min(buf.len(), self.limit);
        let n = self.inner.read(&mut buf[..max])?;
        self.limit -= n;

        Ok(n)
    }
}

impl fmt::Debug for Take<'_>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("Take")
            .field("limit", &self.limit)
            .field("inner", &"dyn <std::io::Read>")
            .finish()
    }
}

#[cfg(test)]
mod tests
{
    use std::io::Cursor;

    use super::*;
    use crate::reader::test_util::test_reader;

    fn str_to_owned_reader(data: &str) -> OwnedReader
    {
        let read = Cursor::new(data.as_bytes().to_vec());

        OwnedReader::new(read)
    }

    test_reader! {str_to_owned_reader}
}
