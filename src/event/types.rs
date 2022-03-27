/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Contains the various types used by [Event]s to represent
//! YAML.

use std::{array::IntoIter as ArrayIter, borrow::Cow, collections::HashMap};

use crate::{
    scanner::{entry::Lazy, error::ScanResult},
    token::Token,
};

pub type Slice<'a> = std::borrow::Cow<'a, str>;

pub const DEFAULT_TAGS: [(Slice<'static>, Slice<'static>); 2] = [
    (Cow::Borrowed("!"), Cow::Borrowed("!")),
    (Cow::Borrowed("!!"), Cow::Borrowed("tag:yaml.org,2002:")),
];
pub const DEFAULT_VERSION: VersionDirective = VersionDirective { major: 1, minor: 2 };
pub const EMPTY_SCALAR: ScalarLike<'static> = ScalarLike::empty();

/// Specific YAML productions found in the YAML stream. Each
/// Event has a start and end mark indicating an approximate
/// range that is represented by the given Event. See
/// [EventData] for all of the possible Event variants.
#[derive(Debug, Clone, PartialEq)]
pub struct Event<'de>
{
    start_mark: usize,
    end_mark:   usize,
    inner:      EventData<'de>,
}

impl<'de> Event<'de>
{
    pub fn new(start_mark: usize, end_mark: usize, event: EventData<'de>) -> Self
    {
        Self {
            start_mark,
            end_mark,
            inner: event,
        }
    }

    pub fn start(&self) -> usize
    {
        self.start_mark
    }

    pub fn end(&self) -> usize
    {
        self.end_mark
    }

    pub fn data(&self) -> &EventData<'de>
    {
        &self.inner
    }

    pub fn data_mut(&mut self) -> &mut EventData<'de>
    {
        &mut self.inner
    }
}

/// The possible variants of an [Event]. See the
/// documentation on each variant for an explanation of what
/// each variant represents.
#[derive(Debug, Clone, PartialEq)]
pub enum EventData<'de>
{
    /// Beginning of event stream, always the first token
    /// produced, and only will be produced once per
    /// event stream
    StreamStart(StreamStart),
    /// End of events, always the last event produced, and
    /// no more events will be produced after seeing this
    /// token
    StreamEnd,

    /// Start of document content, once seen, all future
    /// events belong to this document's scope,
    /// and any tag resolution or version specific behavior
    /// should use the attached directives
    DocumentStart(DocumentStart<'de>),
    /// End of document content, this event may be followed
    /// either by another DocumentStart, or
    /// StreamEnd event.
    DocumentEnd(DocumentEnd),

    /// An alias point connected to a previously seen
    /// `Scalar`, `MappingStart`, or `SequenceStart`
    /// [Node].anchor, the caller must keep track of
    /// this information
    Alias(Alias<'de>),
    /// A scalar leaf node, containing (perhaps lazy)
    /// unicode slice content
    Scalar(Node<'de, ScalarLike<'de>>),

    /// Start of a YAML key value production, followed by
    /// zero or more of:
    ///
    /// `[ MappingStart, SequenceStart, Scalar, Anchor ]`
    ///
    /// until a `MappingEnd` is reached
    MappingStart(Node<'de, Mapping>),
    /// End of a YAML key value production
    MappingEnd,
    /// Start of a YAML array production, followed by zero
    /// or more of:
    ///
    /// `[ MappingStart, SequenceStart, Scalar, Anchor ]`
    ///
    /// until a `SequenceEnd` is reached
    SequenceStart(Node<'de, Sequence>),
    /// End of a YAML array production
    SequenceEnd,
}

/// Wrapper around [Event] variants that correspond to a
/// YAML node production -- that is, those that may have
/// associated tags or aliases.
///
/// One of:
///
///   `[Scalar, MappingStart, SequenceStart]`
#[derive(Debug, Clone, PartialEq)]
pub struct Node<'de, T: 'de>
{
    /// The alias applied to this node (if any)
    pub anchor:  Option<Slice<'de>>,
    /// The tag applied to this node (if any)
    pub tag:     Option<(Slice<'de>, Slice<'de>)>,
    /// The node's content if simple, or a hint about the
    /// complex structure type
    pub content: T,
    /// Contextual information about this Node
    pub kind:    NodeKind,
}

/// Representation of a YAML scalar node, either eagerly
/// evaluated and thus immediately available or lazily
/// evaluated, in which case a caller may trigger a fallible
/// evaluation on demand.
#[derive(Debug, Clone)]
pub enum ScalarLike<'de>
{
    Eager(Scalar<'de>),
    Lazy(ScalarLazy<'de>),
}

impl<'de> ScalarLike<'de>
{
    pub fn evaluate(self) -> Result<Scalar<'de>, crate::Error>
    {
        self.evaluate_scalar().map_err(Into::into)
    }

    pub fn evaluate_by_ref(&mut self) -> Result<&mut Scalar<'de>, crate::Error>
    {
        self.evaluate_scalar_by_ref().map_err(Into::into)
    }

    pub fn is_evaluated(&self) -> bool
    {
        !self.is_lazy()
    }

    pub fn is_unevaluated(&self) -> bool
    {
        self.is_lazy()
    }

    pub(crate) fn evaluate_scalar_by_ref(&mut self) -> ScanResult<&mut Scalar<'de>>
    {
        let this = std::mem::take(self);

        *self = Self::Eager(this.evaluate_scalar()?);

        match self
        {
            ScalarLike::Eager(scalar) => Ok(scalar),
            _ => unreachable!(),
        }
    }

    pub(crate) fn evaluate_scalar(self) -> ScanResult<Scalar<'de>>
    {
        match self
        {
            Self::Eager(scalar) => Ok(scalar),
            Self::Lazy(lazy) => lazy.evaluate_scalar(),
        }
    }

    pub(crate) fn eager(data: Slice<'de>, style: ScalarStyle) -> Self
    {
        Self::Eager(Scalar { data, style })
    }

    pub(crate) fn lazy(lazy: Lazy<'de>) -> Self
    {
        Self::Lazy(ScalarLazy { inner: lazy })
    }

    const fn is_lazy(&self) -> bool
    {
        matches!(self, Self::Lazy(_))
    }
}

impl ScalarLike<'static>
{
    pub const fn empty() -> Self
    {
        Self::Eager(Scalar {
            data:  Slice::Borrowed(""),
            style: ScalarStyle::Plain,
        })
    }
}

impl Default for ScalarLike<'_>
{
    fn default() -> Self
    {
        ScalarLike::empty()
    }
}

impl<'de> PartialEq for ScalarLike<'de>
{
    fn eq(&self, other: &Self) -> bool
    {
        match (self, other)
        {
            (Self::Eager(s), Self::Eager(o)) => s == o,
            // No ordering is established between yet-to-be-evaluated scalars
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scalar<'de>
{
    data:  Slice<'de>,
    style: ScalarStyle,
}

impl<'de> Scalar<'de>
{
    pub fn data(&self) -> &Slice
    {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut Slice<'de>
    {
        &mut self.data
    }

    pub fn style(&self) -> ScalarStyle
    {
        self.style
    }
}

impl<'de> AsRef<str> for Scalar<'de>
{
    fn as_ref(&self) -> &str
    {
        &*self.data
    }
}

impl<'de> std::ops::Deref for Scalar<'de>
{
    type Target = str;

    fn deref(&self) -> &Self::Target
    {
        &*self.data
    }
}

#[derive(Debug, Clone)]
pub struct ScalarLazy<'de>
{
    inner: Lazy<'de>,
}

impl<'de> ScalarLazy<'de>
{
    pub fn evaluate(self) -> Result<Scalar<'de>, crate::Error>
    {
        self.evaluate_scalar().map_err(Into::into)
    }

    pub(crate) fn evaluate_scalar(self) -> ScanResult<Scalar<'de>>
    {
        self.inner.into_token().map(|t| match t
        {
            Token::Scalar(data, style) => Scalar { data, style },
            // Only scalars can be deferred
            _ => unreachable!(),
        })
    }
}

/// Contextual information about this [Node]'s position in
/// the YAML byte stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind
{
    /// Top level [Node] of the YAML document, will only
    /// (and always) be found on the first Node of
    /// each YAML document
    Root,

    /// Entry in a YAML sequence
    ///
    /// Example:
    ///
    ///   ---
    ///   - 'A YAML scalar in a sequence'
    ///   ...
    Entry,

    /// A key in a YAML mapping
    ///
    /// Example:
    ///
    ///   ---
    ///   A YAML key: "..."
    /// # ^^^^^^^^^^
    ///   ...
    Key,
    /// A value in a YAML mapping
    ///
    /// Example:
    ///
    ///   ---
    ///   "...": "A YAML value"
    /// #         ^^^^^^^^^^^^
    ///   ...
    Value,
}

/// StreamStart [Event] contents
#[derive(Debug, Clone, PartialEq)]
pub struct StreamStart
{
    /// Encoding used in the YAML byte stream
    pub encoding: StreamEncoding,
}

/// DocumentStart [Event] contents
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentStart<'de>
{
    pub directives: Directives<'de>,
    /// Was this event present in the stream, or inferred?
    pub implicit:   bool,
}

/// DocumentEnd [Event] contents
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentEnd
{
    /// Was this event present in the stream, or inferred?
    pub implicit: bool,
}

/// Anchor [Event] contents
#[derive(Debug, Clone, PartialEq)]
pub struct Alias<'de>
{
    /// Name of the alias this anchor refers to.
    pub name: Slice<'de>,
}

/// MappingStart [Event] stub
#[derive(Debug, Clone, PartialEq)]
pub struct Mapping;
/// SequenceStart [Event] stub
#[derive(Debug, Clone, PartialEq)]
pub struct Sequence;

/// YAML Directives belonging to a document
#[derive(Debug, Clone, PartialEq)]
pub struct Directives<'de>
{
    /// %YAML directive, indicating the YAML schema version
    /// used in for the current document
    pub version: VersionDirective,
    /// Map of %TAG directives found in the stream
    pub tags:    TagDirectives<'de>,
}

impl<'de> Directives<'de>
{
    pub fn empty() -> Self
    {
        Self {
            version: DEFAULT_VERSION,
            tags:    TagDirectives::new(),
        }
    }
}

impl Default for Directives<'_>
{
    fn default() -> Self
    {
        Self {
            version: DEFAULT_VERSION,
            tags:    ArrayIter::new(DEFAULT_TAGS).collect(),
        }
    }
}

/// %YAML directive representation, containing the .major
/// and .minor version of the current document in the YAML
/// stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionDirective
{
    pub major: u8,
    pub minor: u8,
}

/// Typedef map of tag directives present in the current
/// document
pub type TagDirectives<'de> = HashMap<Slice<'de>, Slice<'de>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamEncoding
{
    UTF8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarStyle
{
    Plain,
    SingleQuote,
    DoubleQuote,
    Literal,
    Folded,
}
