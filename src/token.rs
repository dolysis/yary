/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub type Slice<'a> = std::borrow::Cow<'a, str>;

/// Tokens that may be emitted by a YAML scanner
#[derive(Debug, PartialEq)]
pub enum Token<'a>
{
    /// The stream's start, with the byte (encoding)
    /// {virtual}
    StreamStart(StreamEncoding),
    /// The stream's end {virtual}
    StreamEnd,
    /// The %YAML directive, (major,minor)
    VersionDirective(u8, u8),
    /// The %TAG directive
    TagDirective(Slice<'a>, Slice<'a>),
    /// A ---
    DocumentStart,
    /// A ...
    DocumentEnd,
    /// Indentation increase for a block (sequence)
    BlockSequenceStart,
    /// Indentation increase for a block (map)
    BlockMappingStart,
    /// Indentation decrease for a block
    BlockEnd,
    /// A '['
    FlowSequenceStart,
    /// A ']'
    FlowSequenceEnd,
    /// A '{'
    FlowMappingStart,
    /// A '}'
    FlowMappingEnd,
    /// A '-'
    BlockEntry,
    /// A ','
    FlowEntry,
    /// Either a '?' or nothing
    Key,
    /// A ':'
    Value,
    /// An alias (*anchor)
    Alias(Slice<'a>),
    /// An anchor (&anchor)
    Anchor(Slice<'a>),
    /// A tag (!handle, !suffix)
    Tag(Slice<'a>, Slice<'a>),
    /// A scalar (value, style)
    Scalar(Slice<'a>, ScalarStyle),
}

impl<'a> Token<'a>
{
    pub fn into_owned(self) -> Token<'static>
    {
        match self
        {
            Token::StreamStart(encoding) => Token::StreamStart(encoding),
            Token::StreamEnd => Token::StreamEnd,
            Token::VersionDirective(major, minor) => Token::VersionDirective(major, minor),
            Token::TagDirective(handle, suffix) => Token::TagDirective(
                Slice::Owned(handle.into_owned()),
                Slice::Owned(suffix.into_owned()),
            ),
            Token::DocumentStart => Token::DocumentStart,
            Token::DocumentEnd => Token::DocumentEnd,
            Token::BlockSequenceStart => Token::FlowSequenceStart,
            Token::BlockMappingStart => Token::FlowMappingStart,
            Token::BlockEnd => Token::BlockEnd,
            Token::FlowSequenceStart => Token::FlowSequenceStart,
            Token::FlowSequenceEnd => Token::FlowSequenceEnd,
            Token::FlowMappingStart => Token::FlowMappingStart,
            Token::FlowMappingEnd => Token::FlowMappingEnd,
            Token::BlockEntry => Token::BlockEntry,
            Token::FlowEntry => Token::FlowEntry,
            Token::Key => Token::Key,
            Token::Value => Token::Value,
            Token::Alias(alias) => Token::Alias(Slice::Owned(alias.into_owned())),
            Token::Anchor(anchor) => Token::Anchor(Slice::Owned(anchor.into_owned())),
            Token::Tag(handle, suffix) => Token::Tag(
                Slice::Owned(handle.into_owned()),
                Slice::Owned(suffix.into_owned()),
            ),
            Token::Scalar(contents, kind) =>
            {
                Token::Scalar(Slice::Owned(contents.into_owned()), kind)
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker
{
    /// The stream's start, with the byte (encoding)
    /// {virtual}
    StreamStart,
    /// The stream's end {virtual}
    StreamEnd,
    /// The %YAML directive, (major,minor)
    VersionDirective,
    /// The %TAG directive
    TagDirective,
    /// A ---
    DocumentStart,
    /// A ...
    DocumentEnd,
    /// Indentation increase for a block (sequence)
    BlockSequenceStart,
    /// Indentation increase for a block (map)
    BlockMappingStart,
    /// Indentation decrease for a block
    BlockEnd,
    /// A '['
    FlowSequenceStart,
    /// A ']'
    FlowSequenceEnd,
    /// A '{'
    FlowMappingStart,
    /// A '}'
    FlowMappingEnd,
    /// A '-'
    BlockEntry,
    /// A ','
    FlowEntry,
    /// Either a '?' or nothing
    Key,
    /// A ':'
    Value,
    /// An alias (*anchor)
    Alias,
    /// An anchor (&anchor)
    Anchor,
    /// A tag (!handle, !suffix)
    Tag,
    /// A scalar (value, style)
    Scalar,
}

impl Marker
{
    fn from_token(t: &Token<'_>) -> Self
    {
        use Token::*;

        match t
        {
            StreamStart(_) => Self::StreamStart,
            StreamEnd => Self::StreamEnd,
            VersionDirective(_, _) => Self::VersionDirective,
            TagDirective(_, _) => Self::TagDirective,
            DocumentStart => Self::DocumentStart,
            DocumentEnd => Self::DocumentEnd,
            BlockSequenceStart => Self::BlockSequenceStart,
            BlockMappingStart => Self::BlockMappingStart,
            BlockEnd => Self::BlockEnd,
            FlowSequenceStart => Self::FlowSequenceStart,
            FlowSequenceEnd => Self::FlowSequenceEnd,
            FlowMappingStart => Self::FlowMappingStart,
            FlowMappingEnd => Self::FlowMappingEnd,
            BlockEntry => Self::BlockEntry,
            FlowEntry => Self::FlowEntry,
            Key => Self::Key,
            Value => Self::Value,
            Alias(_) => Self::Alias,
            Anchor(_) => Self::Anchor,
            Tag(_, _) => Self::Tag,
            Scalar(_, _) => Self::Scalar,
        }
    }
}

impl Default for Marker
{
    fn default() -> Self
    {
        Self::StreamStart
    }
}

impl From<&'_ Token<'_>> for Marker
{
    fn from(t: &'_ Token<'_>) -> Self
    {
        Self::from_token(t)
    }
}

impl PartialEq<Token<'_>> for Marker
{
    fn eq(&self, other: &Token<'_>) -> bool
    {
        self == &Self::from(other)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamEncoding
{
    UTF8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarStyle
{
    Plain,
    SingleQuote,
    DoubleQuote,
    Literal,
    Folded,
}
