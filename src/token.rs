pub type Slice<'a> = std::borrow::Cow<'a, str>;

/// Tokens that may be emitted by a YAML scanner
#[derive(Debug, PartialEq)]
pub enum Token<'a>
{
    /// The stream's start, with the byte (encoding)
    /// [virtual]
    StreamStart(StreamEncoding),
    /// The stream's end [virtual]
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

#[derive(Debug, PartialEq)]
pub enum StreamEncoding
{
    UTF8,
}

#[derive(Debug, PartialEq)]
pub enum ScalarStyle
{
    Plain,
    SingleQuote,
    DoubleQuote,
    Literal,
    Folded,
}
