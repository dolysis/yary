/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! This module exposes methods for directly interacting
//! with YAML event streams.
//!
//! ## Understanding Events
//!
//! Each event produced represents an important semantic
//! change in the underlying YAML byte stream. Broadly,
//! these can be categorized into three spaces:
//!
//! 1. Virtual / Marker
//!     - [`StreamStart`]
//!     - [`StreamEnd`]
//!     - [`DocumentStart`]
//!     - [`DocumentEnd`]
//!
//! 2. Nesting change (+-)
//!     - [`MappingStart`]
//!     - [`MappingEnd`]
//!     - [`SequenceStart`]
//!     - [`SequenceEnd`]
//!
//! 3. Data / Alias
//!     - [`Scalar`]
//!     - [`Alias`]
//!
//! Together, these are used to produce the following
//! productions:
//!
//! ```text
//! stream          := StreamStart document+ StreamEnd
//! document        := DocumentStart content? DocumentEnd
//! content         := Scalar | collection
//! collection      := sequence | mapping
//! sequence        := SequenceStart node* SequenceEnd
//! mapping         := MappingStart (node node)* MappingEnd
//! node            := Alias | content
//!
//! ?               => 0 or 1 of prefix
//! *               => 0 or more of prefix
//! +               => 1 or more of prefix
//! ()              => production grouping
//! |               => production logical OR
//! ```
//!
//! In addition to the various [`Event`] types, every
//! [`Node`] also provides a hint as to its placement in the
//! stream via its [`NodeKind`]. Together, these should
//! allow users to maintain relatively little external state
//! regarding the [`Event`] stream, beyond anything they
//! wish to collect from the stream.
//!
//! [`StreamStart`]:    type@types::EventData::StreamStart
//! [`StreamEnd`]:      type@types::EventData::StreamEnd
//! [`DocumentStart`]:  type@types::EventData::DocumentStart
//! [`DocumentEnd`]:    type@types::EventData::DocumentEnd
//! [`MappingStart`]:   type@types::EventData::MappingStart
//! [`MappingEnd`]:     type@types::EventData::MappingEnd
//! [`SequenceStart`]:  type@types::EventData::SequenceStart
//! [`SequenceEnd`]:    type@types::EventData::SequenceEnd
//! [`Scalar`]:         type@types::EventData::Scalar
//! [`Alias`]:          type@types::EventData::Alias
//! [`Node`]:           struct@types::Node
//! [`NodeKind`]:       enum@types::NodeKind
//! [`Token`]:          enum@crate::token::Token
//! [`Read`]:           trait@crate::reader::Read

use crate::{
    error::Result,
    event::{
        error::ParseResult,
        flag::{Flags, O_NIL},
        parser::Parser,
        types::Event,
    },
    reader::{PeekReader, Reader},
};

mod parser;
mod state;

pub(crate) mod error;

pub mod flag;
pub mod types;

/// Instantiates a new [`Events`] stream from the given
/// read source, with the default flag set.
///
/// See [`from_reader_with`] for more information.
///
/// ## Examples
///
/// ```rust
/// use yary::{
///     event::{from_reader, types::EventData},
///     reader::borrow::BorrowReader,
/// };
///
/// let yaml = BorrowReader::new("[a yaml, event sequence]");
/// let events = from_reader(&yaml);
///
/// let expected: Vec<fn(&EventData) -> bool> = vec![
///     // Start of stream housekeeping
///     |e| matches!(e, EventData::StreamStart(_)),
///     |e| matches!(e, EventData::DocumentStart(_)),
///     // '['
///     |e| matches!(e, EventData::SequenceStart(_)),
///     // 'a yaml' 'event sequence'
///     |e| matches!(e, EventData::Scalar(_)),
///     |e| matches!(e, EventData::Scalar(_)),
///     // ']'
///     |e| matches!(e, EventData::SequenceEnd),
///     // End of stream housekeeping
///     |e| matches!(e, EventData::DocumentEnd(_)),
///     |e| matches!(e, EventData::StreamEnd),
/// ];
///
/// for (event, f) in events.into_iter().filter_map(Result::ok).zip(expected)
/// {
///     let matched = f(event.data());
///
///     assert!(matched);
/// }
/// ```
pub fn from_reader<R>(src: &R) -> Events<'_, R>
where
    R: crate::reader::Read,
{
    from_reader_with(src, O_NIL)
}

/// Instantiates a new [`Events`] stream from the given
/// read source and flags.
///
/// The [`Event`]s produced by this stream will have their
/// lifetime bound to the source reference, and will remain
/// valid for as long as this `'de` reference is live.
///
/// **Note:** After you have generated >=1 [`Event`]s from
/// the returned [`Events`]' iterator, this read source is
/// "bound", and will error if provided to any other
/// [`yary::event`][self] functions.
///
/// ## Examples
///
/// ```rust
/// use yary::{
///     event::{flag::*, from_reader_with, types::EventData},
///     reader::borrow::BorrowReader,
/// };
///
/// let yaml = BorrowReader::new("{a yaml: mapping}");
/// let events = from_reader_with(&yaml, O_NIL | O_LAZY);
///
/// let expected: Vec<fn(&EventData) -> bool> = vec![
///     // Start of stream housekeeping
///     |e| matches!(e, EventData::StreamStart(_)),
///     |e| matches!(e, EventData::DocumentStart(_)),
///     // '{'
///     |e| matches!(e, EventData::MappingStart(_)),
///     // 'a yaml' 'mapping'
///     |e| matches!(e, EventData::Scalar(_)),
///     |e| matches!(e, EventData::Scalar(_)),
///     // '}'
///     |e| matches!(e, EventData::MappingEnd),
///     // End of stream housekeeping
///     |e| matches!(e, EventData::DocumentEnd(_)),
///     |e| matches!(e, EventData::StreamEnd),
/// ];
///
/// for (event, f) in events.into_iter().filter_map(Result::ok).zip(expected)
/// {
///     let matched = f(event.data());
///
///     assert!(matched);
/// }
/// ```
pub fn from_reader_with<R>(src: &R, f: Flags) -> Events<'_, R>
where
    R: crate::reader::Read,
{
    Events::new(src, f)
}

/// Control structure for [`Event`] production, providing an
/// iterator based API for consuming events.
///
/// The returned events have a lifetime associated with the
/// `'de` lifetime of the backing source, independent from
/// this structure. In practice, this means that this
/// structure should be considered ephemeral, and discarded
/// once event production is complete.
///
/// It's primary usage is as an iterator, either by
/// reference, via [`iter`](#method.iter), or by
/// value with [`into_iter`](#method.into_iter).
#[derive(Debug)]
pub struct Events<'de, R>
{
    reader: PeekReader<'de, R>,
    parser: Parser,
}

impl<'de, R> Events<'de, R>
where
    R: crate::reader::Read,
{
    /// Instantiate a new [`Events`] from the given read
    /// source and flags.
    ///
    /// **Note:** After you have generated >=1 [`Event`]s
    /// from the returned [`Events`]' iterator, this
    /// read source is "bound", and will error if
    /// provided to any other [`yary::event`][self]
    /// functions.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use yary::reader::borrow::BorrowReader;
    /// # use yary::event::{Events, flag::O_NIL};
    ///
    /// let yaml = BorrowReader::new("[1, 2, 3, 4, 5]");
    ///
    /// let events = Events::new(&yaml, O_NIL);
    /// ```
    pub fn new(src: &'de R, flags: Flags) -> Self
    {
        let inner = Reader::new(src, flag::as_scanner(flags));
        let reader = PeekReader::new(inner);
        let parser = Parser::new();

        Self { reader, parser }
    }

    /// Return an iterator which borrows from the underlying
    /// [`Events`] structure.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use yary::reader::borrow::BorrowReader;
    /// # use yary::event::{Events, flag::O_NIL, types::*};
    ///
    /// let yaml = BorrowReader::new("[1, 2, 3, 4, 5]");
    /// let mut events = Events::new(&yaml, O_NIL);
    ///
    /// // Print out any numbers in the YAML stream
    /// for e in events.iter().filter_map(Result::ok)
    /// {
    ///     if let EventData::Scalar(Node {
    ///         content: ScalarLike::Eager(scalar),
    ///         ..
    ///     }) = e.data()
    ///     {
    ///         if let Ok(digit) = scalar.parse::<i32>()
    ///         {
    ///             println!("Got digit: {}", digit);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn iter<'a>(&'a mut self) -> EventIterRef<'a, 'de, R>
    {
        EventIterRef::new(self)
    }
}

impl<'de, R> IntoIterator for Events<'de, R>
where
    R: crate::reader::Read,
{
    type Item = Result<Event<'de>>;

    type IntoIter = EventIter<'de, R>;

    fn into_iter(self) -> Self::IntoIter
    {
        EventIter::new(self)
    }
}

/// Owning iterator over an underlying [`Events`].
///
/// It is unlikely you want to construct this type by hand.
/// Consider using the [`IntoIterator`] implementation on
/// [`Events`] instead.
#[derive(Debug)]
pub struct EventIter<'de, R>
{
    inner: Events<'de, R>,
}

impl<'de, R> EventIter<'de, R>
where
    R: crate::reader::Read,
{
    /// Instantiate an owning [`Events`] iterator
    pub fn new(inner: Events<'de, R>) -> Self
    {
        Self { inner }
    }

    /// Retrieve the underlying [`Events`], discarding this
    /// iterator
    pub fn into_inner(self) -> Events<'de, R>
    {
        self.inner
    }

    /// Process the next event
    pub(crate) fn next_event(&mut self) -> ParseResult<Option<Event<'de>>>
    {
        self.inner
            .parser
            .next_event(&mut self.inner.reader)
            .transpose()
    }
}

impl<'de, R> Iterator for EventIter<'de, R>
where
    R: crate::reader::Read,
{
    type Item = Result<Event<'de>>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.next_event().map_err(Into::into).transpose()
    }
}

/// Borrowing iterator over an underlying [`Events`].
///
/// It is unlikely you want to construct this type by hand.
/// Consider using [`Events::iter()`] instead.
#[derive(Debug)]
pub struct EventIterRef<'a, 'de, R>
{
    inner: &'a mut Events<'de, R>,
}

impl<'a, 'de, R> EventIterRef<'a, 'de, R>
where
    R: crate::reader::Read,
{
    /// Instantiate a borrowing [`Events`] iterator
    pub fn new(parent: &'a mut Events<'de, R>) -> Self
    {
        Self { inner: parent }
    }

    /// Process the next event
    pub(crate) fn next_event(&mut self) -> ParseResult<Option<Event<'de>>>
    {
        self.inner
            .parser
            .next_event(&mut self.inner.reader)
            .transpose()
    }
}

impl<'a, 'de, R> Iterator for EventIterRef<'a, 'de, R>
where
    R: crate::reader::Read,
{
    type Item = Result<Event<'de>>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.next_event().map_err(Into::into).transpose()
    }
}
