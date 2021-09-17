/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! The Queue is a stable min heap structure that uses std's
//! BinaryHeap at it's core. This means that is has ~O(1)
//! insert, and O(log(n)) pop operations.
//!
//! While it does have a worst case O(n) pop if the data is
//! pathological, we will _mostly_ be inserting elements in
//! sorted order, only occasionally requiring out of order
//! inserts, and never more than +-3 elements apart.

use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    fmt::{self, Debug},
    iter::FromIterator,
};

/// A min heap data structure that keeps a stable ordering
/// of elements, ensuring that otherwise equal items are
/// returned in the order added
pub(crate) struct Queue<T>
{
    heap:      BinaryHeap<Reverse<QueueEntry<T>>>,
    increment: usize,
}

impl<T> Queue<T>
where
    T: Ord,
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn push(&mut self, item: T)
    {
        let entry = QueueEntry::new(self.increment(), item);

        self.heap.push(Reverse(entry))
    }

    pub fn pop(&mut self) -> Option<T>
    {
        if let Some(Reverse(QueueEntry { entry, mark: _ })) = self.heap.pop()
        {
            return Some(entry);
        };

        None
    }

    pub fn sort(&mut self)
    {
        let heap = std::mem::take(&mut self.heap);
        let heap = heap.into_sorted_vec();

        self.heap = BinaryHeap::from(heap);
    }

    pub fn into_sorted_vec(self) -> Vec<T>
    {
        self.into_iter().collect()
    }

    pub fn len(&self) -> usize
    {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.heap.is_empty()
    }

    pub fn capacity(&self) -> usize
    {
        self.heap.capacity()
    }

    pub fn reserve(&mut self, additional: usize)
    {
        self.heap.reserve(additional)
    }

    fn increment(&mut self) -> usize
    {
        self.increment += 1;

        self.increment
    }
}

impl<T> IntoIterator for Queue<T>
where
    T: Ord,
{
    type Item = T;

    type IntoIter = QueueIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter
    {
        Self::IntoIter::new(self)
    }
}

impl<T> Default for Queue<T>
where
    T: Ord,
{
    fn default() -> Self
    {
        Queue {
            heap:      Default::default(),
            increment: 0,
        }
    }
}

impl<T> From<Vec<T>> for Queue<T>
where
    T: Ord,
{
    fn from(v: Vec<T>) -> Self
    {
        Self::from_iter(v)
    }
}

impl<T> FromIterator<T> for Queue<T>
where
    T: Ord,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self
    {
        let iter = iter.into_iter();
        let capacity = match iter.size_hint()
        {
            (_, Some(upper)) => upper,
            (lower, None) => lower,
        };

        let mut heap = BinaryHeap::with_capacity(capacity);
        let mut increment = 0;

        iter.for_each(|item| {
            increment += 1;
            heap.push(Reverse(QueueEntry::new(increment, item)))
        });

        Self { heap, increment }
    }
}

impl<T> Clone for Queue<T>
where
    T: Clone,
{
    fn clone(&self) -> Self
    {
        Self {
            heap:      self.heap.clone(),
            increment: self.increment,
        }
    }
}

impl<T> Debug for Queue<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_list().entries(self.heap.iter()).finish()
    }
}

pub(crate) struct QueueIntoIter<T>
{
    inner: Queue<T>,
}

impl<T> QueueIntoIter<T>
where
    T: Ord,
{
    pub fn new(q: Queue<T>) -> Self
    {
        Self { inner: q }
    }

    pub fn into_inner(self) -> Queue<T>
    {
        self.inner
    }
}

impl<T> Iterator for QueueIntoIter<T>
where
    T: Ord,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.inner.pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        let exact = self.inner.len();

        (exact, Some(exact))
    }
}

/// Entry wrapper that ensures when an entry's ordering is
/// equal a tie breaker is held via mark
struct QueueEntry<T>
{
    entry: T,
    mark:  usize,
}

impl<T> QueueEntry<T>
{
    pub fn new(mark: usize, entry: T) -> Self
    {
        Self { entry, mark }
    }
}

impl<T> PartialEq for QueueEntry<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool
    {
        self.entry.eq(&other.entry)
    }
}

impl<T> Eq for QueueEntry<T> where T: Eq {}

impl<T> PartialOrd for QueueEntry<T>
where
    T: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

impl<T> Ord for QueueEntry<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        match self.entry.cmp(&other.entry)
        {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.mark.cmp(&other.mark),
        }
    }
}

impl<T> Clone for QueueEntry<T>
where
    T: Clone,
{
    fn clone(&self) -> Self
    {
        let QueueEntry { entry, mark } = self;

        Self::new(*mark, entry.clone())
    }
}

impl<T> Debug for QueueEntry<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("QueueEntry")
            .field("entry", &self.entry)
            .field("mark", &self.mark)
            .finish()
    }
}

#[cfg(test)]
mod tests
{
    use pretty_assertions::assert_eq;

    use super::*;

    macro_rules! t {
        ($msg:expr, $ord:expr) => {
            T::new($msg, $ord)
        };
    }

    #[test]
    fn stable_ordering()
    {
        let data = vec![0, 0, 0, 1, 2, 2];
        let expected = vec!["one", "two", "three", "four", "five", "six"];

        assert!(data.len() == expected.len());

        let test = data
            .into_iter()
            .zip(expected.iter())
            .fold(Queue::new(), |mut q, (num, msg)| {
                q.push(t!(msg, num));
                q
            });

        for (T { msg, ord }, expected) in test.into_iter().zip(expected)
        {
            assert_eq!(
                expected, msg,
                "Expected stable ordering for '{}', got '{}' (number: {})",
                expected, msg, ord
            );
        }
    }

    #[derive(Debug, Clone)]
    struct T
    {
        msg: &'static str,
        ord: isize,
    }

    impl T
    {
        fn new(msg: &'static str, ord: isize) -> Self
        {
            Self { msg, ord }
        }
    }

    impl PartialEq for T
    {
        fn eq(&self, other: &T) -> bool
        {
            self.ord == other.ord
        }
    }

    impl Eq for T {}

    impl PartialOrd for T
    {
        fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering>
        {
            self.ord.partial_cmp(&other.ord)
        }
    }

    impl Ord for T
    {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering
        {
            self.ord.cmp(&other.ord)
        }
    }
}
