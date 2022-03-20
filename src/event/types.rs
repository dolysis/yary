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

/// Copy on write representation of YAML data content.
///
/// Currently, it is a typedef of [`Cow`](std::borrow::Cow),
/// though this will change in the future.
///
/// Most variable data returned in [`Event`]s will be stored
/// as this type, and where possible, will be borrowed from
/// the underlying byte stream.
pub type Slice<'a> = std::borrow::Cow<'a, str>;

/// Default tag directives applied to every YAML document.
///
/// Equivalent of:
///
/// ```yaml
/// %TAG !    !
/// %TAG !!   tag:yaml.org,2002:
/// ```
///
/// These are always in scope, though documents may
/// expressly override them
pub const DEFAULT_TAGS: [(Slice<'static>, Slice<'static>); 2] = [
    (Cow::Borrowed("!"), Cow::Borrowed("!")),
    (Cow::Borrowed("!!"), Cow::Borrowed("tag:yaml.org,2002:")),
];

/// Default YAML stream version. If unspecified in the
/// stream it is assumed to be this version.
///
/// Equivalent of:
///
/// ```yaml
/// %YAML 1.2
/// ```
pub const DEFAULT_VERSION: VersionDirective = VersionDirective { major: 1, minor: 2 };

/// An empty YAML scalar.
///
/// In many circumstances, a YAML node is implied by the
/// stream state, though it may not appear in the stream
/// directly. This constant is the representation of such
/// nodes.
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
    /// Instantiate a new [`Event`] with the given marks and
    /// data
    pub fn new(start_mark: usize, end_mark: usize, event: EventData<'de>) -> Self
    {
        Self {
            start_mark,
            end_mark,
            inner: event,
        }
    }

    /// Retrieve the start mark of this [`Event`]
    pub fn start(&self) -> usize
    {
        self.start_mark
    }

    /// Retrieve the end mark of this [`Event`]
    pub fn end(&self) -> usize
    {
        self.end_mark
    }

    /// Retrieve the data associated with this [`Event`]
    pub fn data(&self) -> &EventData<'de>
    {
        &self.inner
    }

    /// Retrieve the data associated with this [`Event`]
    /// mutably
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
    /// Evaluated scalar, whose contents can be accessed
    Eager(Scalar<'de>),
    /// Unevaluated potential scalar, which must be
    /// processed before its contents can be accessed.
    ///
    /// See [`evaluate()`](#method.evaluate) for more.
    Lazy(ScalarLazy<'de>),
}

impl<'de> ScalarLike<'de>
{
    /// Evaluate this [`ScalarLike`], returning the
    /// underlying [`Scalar`].
    ///
    /// ## Errors
    ///
    /// This method may error if `self == Self::Lazy(_)` and
    /// the underlying scalar is invalid.
    pub fn evaluate(self) -> Result<Scalar<'de>, crate::Error>
    {
        self.evaluate_scalar().map_err(Into::into)
    }

    /// Evaluate this [`ScalarLike`] by reference, returning
    /// a reference to the underlying [`Scalar`].
    ///
    /// After calling this method it is guaranteed that
    /// `self == Self::Eager(_)`, though if an error was
    /// returned the scalar will be empty
    ///
    /// ## Errors
    ///
    /// This method may error if `self == Self::Lazy(_)` and
    /// the underlying scalar is invalid.
    pub fn evaluate_by_ref(&mut self) -> Result<&mut Scalar<'de>, crate::Error>
    {
        self.evaluate_scalar_by_ref().map_err(Into::into)
    }

    /// Checks if this scalar has been evaluated
    ///
    /// If this returns true it is guaranteed that `self ==
    /// Self::Eager(_)`
    pub fn is_evaluated(&self) -> bool
    {
        !self.is_lazy()
    }

    /// Checks if this scalar has not been evaluated
    ///
    /// If this returns true it is guaranteed that `self ==
    /// Self::Lazy(_)`
    pub fn is_unevaluated(&self) -> bool
    {
        self.is_lazy()
    }

    /// Private version of `evaluate_by_ref()`,
    /// returning a less expensive error type.
    ///
    /// Library local callers should use this variant, and
    /// convert the error lazily as needed.
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

    /// Private version of `evaluate`, returning a less
    /// expensive error type.
    ///
    /// Library local callers should use this variant, and
    /// convert the error lazily as needed.
    pub(crate) fn evaluate_scalar(self) -> ScanResult<Scalar<'de>>
    {
        match self
        {
            Self::Eager(scalar) => Ok(scalar),
            Self::Lazy(lazy) => lazy.evaluate_scalar(),
        }
    }

    /// Initialize a new, Eager variant
    pub(crate) fn eager(data: Slice<'de>, style: ScalarStyle) -> Self
    {
        Self::Eager(Scalar { data, style })
    }

    /// Initialize a new, Lazy variant
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
    /// Instantiate an empty [`ScalarLike`]
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

/// Representation of a YAML Scalar, containing the
/// associated data and style
///
/// This struct implements `Deref<Target = str>`, backed by
/// the underlying data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scalar<'de>
{
    data:  Slice<'de>,
    style: ScalarStyle,
}

impl<'de> Scalar<'de>
{
    /// Retrieve the associated data of this [`Scalar`].
    pub fn data(&self) -> &Slice
    {
        &self.data
    }

    /// Mutably retrieve the associated data of this
    /// [`Scalar`].
    pub fn data_mut(&mut self) -> &mut Slice<'de>
    {
        &mut self.data
    }

    /// Retrieve this [`Scalar`]'s style.
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

/// Opaque wrapper around an unevaluated [`Scalar`].
///
/// Typically, an owner of this struct may make one of two
/// choices:
///
/// 1. Evaluate the scalar, handling the potential error
/// 2. Drop it, permanently deferring computation of the
///    scalar
///
/// This allows owners to potentially avoid paying for
/// processing that is unnecessary if they can decide based
/// on some outside criteria whether to evaluate the
/// underlying scalar.
#[derive(Debug, Clone)]
pub struct ScalarLazy<'de>
{
    inner: Lazy<'de>,
}

impl<'de> ScalarLazy<'de>
{
    /// Consume this struct, retrieving the underlying
    /// [`Scalar`], or an error.
    ///
    /// ## Errors
    ///
    /// This method may error if the contents of the scalar
    /// are either syntactically invalid or if an error was
    /// encountered when trying to represent them as Rust
    /// constructs.
    pub fn evaluate(self) -> Result<Scalar<'de>, crate::Error>
    {
        self.evaluate_scalar().map_err(Into::into)
    }

    /// Private version of `evaluate`, returning a less
    /// expensive error type.
    ///
    /// Library local callers should use this variant, and
    /// convert the error lazily as needed.
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
    /// This document's directive map
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
    /// Instantiate a new, empty directives map
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
            tags:    array_iterator(DEFAULT_TAGS).collect(),
        }
    }
}

/// %YAML directive representation, containing the .major
/// and .minor version of the current document in the YAML
/// stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionDirective
{
    /// The major version of this YAML stream
    pub major: u8,
    /// The minor version of this YAML stream
    pub minor: u8,
}

/// Typedef map of tag directives present in the current
/// document
pub type TagDirectives<'de> = HashMap<Slice<'de>, Slice<'de>>;

/// The encoding of the underlying byte stream.
///
/// Currently, and for the forseeable future only `UTF8`
/// will be supported, though this may change eventually.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamEncoding
{
    /// The byte stream is encoded as UTF8
    UTF8,
}

/// The style of a [`Scalar`], corresponding to the possible
/// styles supported by YAML
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarStyle
{
    /// No delimiters, content was detected purely from
    /// stream context
    Plain,
    /// Scalar was quoted in single quotes (`''`)
    SingleQuote,
    /// Scalar was quoted in double quotes (`""`)
    DoubleQuote,
    /// Scalar was preceded by pipe (`|`)
    Literal,
    /// Scalar was preceded by an arrow ('>')
    Folded,
}

/// Wrapper around IntoIterator::into_iter that works around
/// the hack in `std` which makes our Rust edition's
/// ARRAY.into_iter() postfix call take the array by
/// reference.
///
/// It appears that ArrayIter::new() has been deprecated in
/// a future rust version (1.59), so this should quiet those
/// errors, when building against stable.
///
/// If/when we bump the crate's MSRV to >= 1.59 we can
/// remove this function and call the postfix .into_iter()
/// method directly.
pub(in crate::event) fn array_iterator<T, const N: usize>(arr: [T; N]) -> ArrayIter<T, N>
{
    IntoIterator::into_iter(arr)
}
