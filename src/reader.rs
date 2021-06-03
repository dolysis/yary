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
        let mut done = 0;

        while done < amt {
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    macro_rules! data {
        ($data:expr) => {
            $data.chars().map(Ok)
        };
        () => {
            "1234567890".chars().map(Ok)
        };
    }

    #[test]
    fn amount() {
        let data = data!();
        let r = Reader::new(data);

        assert_eq!(r.into_iter().count(), 10);
    }

    #[test]
    fn reserve() {
        let data = data!();
        let mut r = Reader::new(data);

        let amount = r.reserve(20).expect("impossible to error");

        assert_ne!(amount, 20);
        assert_eq!(amount, 10);
    }

    #[test]
    fn reserve_partial() {
        let data = data!();
        let mut r = Reader::new(data);

        let amount = r.reserve(5).expect("impossible to error");

        assert_eq!(amount, 5);
    }

    #[test]
    fn reserve_one() {
        let data = data!();
        let mut r = Reader::new(data);

        let amount = r.reserve(1).expect("impossible to error");

        assert_eq!(amount, 1);
    }

    #[test]
    fn reserve_zero() {
        let data = data!();
        let mut r = Reader::new(data);

        let amount = r.reserve(0).expect("impossible to error");

        assert_eq!(amount, 0);
    }

    #[test]
    fn column() {
        let data = data!("abc\nefg\nhijkl");
        let expected = vec![1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 4, 5];
        let mut r = Reader::new(data);

        assert_eq!(r.column(), 0);

        for indent in expected {
            let c = r.next().unwrap().expect("impossible to error");

            assert_eq!(r.column(), indent, "@ char: {}, mark: {}", c, r.mark());
        }
    }

    #[test]
    fn mark() {
        let data = data!();
        let expected = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut r = Reader::new(data);

        assert_eq!(r.mark(), 0);

        for mark in expected {
            let c = r.next().unwrap().expect("impossible to error");

            assert_eq!(r.mark(), mark, "@ char: {}, mark: {}", c, r.mark());
        }
    }
}
