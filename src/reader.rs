use std::collections::VecDeque;

use anyhow::bail;

use crate::error::Result;

#[derive(Debug)]
struct Reader<I> {
    src: I,
    buffer: VecDeque<char>,
    column: usize,
    mark: usize,
}

impl<I> Reader<I>
where
    I: Iterator<Item = Result<char>>,
{
    pub fn new(src: I) -> Self {
        Self {
            src,
            buffer: VecDeque::new(),
            column: 0,
            mark: 0,
        }
    }

    /// Number of chars seen so far
    pub fn mark(&self) -> usize {
        self.mark
    }

    /// How deep into a line we currently are
    pub fn column(&self) -> usize {
        self.column
    }

    /// Reserve amt chars, returning an error if it could not reserve the
    /// requested amount
    pub fn reserve_exact(&mut self, amt: usize) -> Result<()> {
        let reserved = self.reserve(amt)?;

        if reserved != amt {
            bail!("only reserved {}/{} chars @{}", reserved, amt, self.mark());
        }

        Ok(())
    }

    /// Attempt to reserve up to amt chars, returning the actual number added
    pub fn reserve(&mut self, amt: usize) -> Result<usize> {
        let mut done = 1;
        while done <= amt {
            match self.src.next() {
                Some(Ok(c)) => self.buffer.push_front(c),
                Some(Err(e)) => return Err(e),
                None => return Ok(done),
            }

            done += 1;
        }

        Ok(done)
    }

    fn next_char(&mut self) -> Result<Option<char>> {
        match self.read_one()? {
            Some('\n') => {
                self.column = 0;
                self.mark += 1;

                Ok(Some('\n'))
            }
            Some(c) => {
                self.column += 1;
                self.mark += 1;

                Ok(Some(c))
            }
            None => Ok(None),
        }
    }

    fn read_one(&mut self) -> Result<Option<char>> {
        match self.buffer.is_empty() {
            true => self.src.next().transpose(),
            false => Ok(self.buffer.pop_back()),
        }
    }
}

impl<I> Iterator for Reader<I>
where
    I: Iterator<Item = Result<char>>,
{
    type Item = Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_char().transpose()
    }
}
