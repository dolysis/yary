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

    pub fn borrowed<'c>(self) -> Ref<'a, 'c>
    {
        Ref::Borrow(self)
    }

    pub fn copied<'b>(self) -> Ref<'b, 'a>
    {
        Ref::Copy(self)
    }
}

/// This allows us to discriminate between a Token with
/// different lifetimes, specifically either a lifetime
/// 'borrow-ed from the underlying data or 'copy-ied from
/// some scratch space provided.
#[derive(Debug, PartialEq)]
pub enum Ref<'borrow, 'copy>
{
    Borrow(Token<'borrow>),
    Copy(Token<'copy>),
}

impl<'b, 'c> Ref<'b, 'c>
{
    /// Unifies the lifetimes of the underlying Token,
    /// returning one that lives at least as long as
    /// 'borrow. Note that this _will_ allocate if a copy
    /// needs to be made.
    pub fn into_inner(self) -> Token<'b>
    {
        match self
        {
            Self::Borrow(t) => t,
            Self::Copy(t) => t.into_owned(),
        }
    }

    /// Short hand check if the Ref contains a borrowed
    /// Token
    pub fn is_borrowed(&self) -> bool
    {
        match self
        {
            Self::Borrow(_) => true,
            Self::Copy(_) => false,
        }
    }

    /// Short hand check if the Ref contains a copied Token
    pub fn is_copied(&self) -> bool
    {
        !self.is_borrowed()
    }
}

impl<'b, 'c> PartialEq<Token<'_>> for Ref<'b, 'c>
{
    fn eq(&self, other: &Token<'_>) -> bool
    {
        match self
        {
            Self::Borrow(t) => t.eq(other),
            Self::Copy(t) => t.eq(other),
        }
    }
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
