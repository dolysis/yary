/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! This module contains the errors that may surface while
//! parsing a YAML byte stream.

use std::{
    fmt::{self, Debug},
    io,
    str::Utf8Error,
};

use crate::{
    error::internal::{ErrorCode, ErrorKind},
    reader::error::ReaderError,
    scanner::error::ScanError,
};

/// Result type returned by [`yary::event`](super)
pub(crate) type ParseResult<T> = std::result::Result<T, ParseError>;

/// Possible errors that can be encountered while parsing
/// YAML events.
#[derive(Debug)]
pub(crate) enum ParseError
{
    /// A mismatch between the stream's actual state and
    /// what the parser is expecting occurred.
    ///
    /// Typically, this only happens if a [`Read`] source is
    /// given to two separate parsers.
    ///
    /// [`Read`]: trait@crate::reader::Read
    CorruptStream,

    /// More than one `%YAML` directive was found inside a
    /// single document's context.
    ///
    /// ## Examples
    ///
    /// ```yaml
    /// ---
    /// %YAML 1.2
    /// %YAML 1.2
    /// #^^^^^^^^ DuplicateVersion
    /// ```
    DuplicateVersion,

    /// More than one `%TAG` directive was found _for the
    /// same handle_ inside a single document's context.
    ///
    /// ## Examples
    ///
    /// ```yaml
    /// ---
    /// %TAG !handle! my:custom:tag/
    /// %TAG !handle! my:other:tag/
    /// #    ^^^^^^^^ DuplicateTagDirective
    /// ```
    DuplicateTagDirective,

    /// A tag referenced a handle that has not been defined.
    ///
    /// ## Examples
    ///
    /// ```yaml
    /// ---
    /// key: !handle! value
    /// #    ^^^^^^^^ UndefinedTag
    /// ```
    UndefinedTag,

    /// In certain cases YAML requires an indication that
    /// another document is being started, necessitating
    /// a DocumentStart '---' symbol.
    ///
    /// Most commonly, if a stream contains two documents
    /// the first must include a DocumentEnd ('...') symbol,
    /// or the second must start with a DocumentStart.
    ///
    /// ## Examples
    ///
    /// ```yaml
    /// document: 1
    /// # <-- either a '...' or '---' is required here
    /// # ^^^^^ MissingDocumentStart
    /// %YAML 1.2
    /// document: 2
    /// ```
    MissingDocumentStart,

    /// A entry in a block sequence was required but not
    /// found in the stream
    ///
    /// ## Examples
    ///
    /// ```yaml
    /// - 1
    /// - 2
    /// - 3
    /// foo: bar # should be: '- foo: bar'
    /// #^^ MissingBlockEntry
    /// ```
    MissingBlockEntry,

    /// A YAML node was required but not found.
    ///
    /// YAML tends to be very forgiving with missing nodes,
    /// but it is possible to construct a YAML stream with a
    /// required Node, most commonly if using tags or
    /// anchors.
    ///
    /// ## Examples
    ///
    /// ```yaml
    /// tag: !!str      # wants: !!str 'my tagged scalar'
    /// #    ^^^^^ MissingNode
    ///
    /// anchor: &anchor # wants: &anchor 'my anchored scalar'
    /// #       ^^^^^^^ MissingNode
    /// ```
    MissingNode,

    /// A YAML mapping key was required but not found.
    ///
    /// Frequently caused by poor indentation in YAML
    /// documents.
    ///
    /// ```yaml
    /// nested:
    ///     key: value
    ///     bad: # YAML requires that a value exist on the same line as its key
    /// #   ^^^^ MissingKey
    ///          value with a different line to key
    /// ```
    ///
    /// ```yaml
    /// some very long key over 1024 bytes long...: value
    /// # ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ MissingKey
    /// ```
    MissingKey,

    /// A YAML flow sequence was missing a terminus (']') or
    /// continuation (',') symbol.
    ///
    /// ## Examples
    ///
    /// ```yaml
    /// missing terminus: [1, 2, 3
    /// #                         ^ MissingFlowSequenceEntryOrEnd
    /// #                           No terminus bracket closing out the sequence
    /// ```
    ///
    /// ```yaml
    /// missing continuation: [ {key1: value} {key2: value} ]
    /// #                                    ^ MissingFlowSequenceEntryOrEnd
    /// ```
    MissingFlowSequenceEntryOrEnd,

    /// A YAML flow mapping was missing a terminus ('}') or
    /// continuation (',') symbol.
    ///
    /// ## Examples
    /// ```yaml
    /// missing terminus: {key1: value, key2: value
    /// #                                          ^ MissingFlowMappingEntryOrEnd
    /// #                                            No terminus bracket closing out the mapping
    /// ```
    ///
    /// ```yaml
    /// missing continuation: {first: value second: value}
    /// #                                  ^ MissingFlowMappingEntryOrEnd
    /// ```
    MissingFlowMappingEntryOrEnd,

    /// The parser was expecting more tokens, but the byte
    /// stream finished unexpectedly.
    UnexpectedEOF,

    /// A issue occurred during stream scanning.
    Scanner(ScanError),

    /// A UTF8 encoded byte stream encountered an encoding
    /// error.
    UTF8(Utf8Error),

    /// An IO error occurred while attempting to read the
    /// byte stream.
    IO(io::Error),
}

impl From<ScanError> for ParseError
{
    fn from(e: ScanError) -> Self
    {
        Self::Scanner(e)
    }
}

impl From<ReaderError> for ParseError
{
    fn from(e: ReaderError) -> Self
    {
        match e
        {
            ReaderError::UTF8(e) => Self::UTF8(e),
            ReaderError::IO(e) => Self::IO(e),
            ReaderError::Scanner(e) => Self::Scanner(e),
        }
    }
}

impl PartialEq for ParseError
{
    fn eq(&self, other: &Self) -> bool
    {
        match (self, other)
        {
            (Self::Scanner(s), Self::Scanner(o)) => s == o,
            (Self::UTF8(s), Self::UTF8(o)) => s == o,
            (Self::IO(s), Self::IO(o)) => s.kind() == o.kind(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl fmt::Display for ParseError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for ParseError
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>
    {
        match self
        {
            Self::Scanner(e) => Some(e),
            Self::UTF8(e) => Some(e),
            Self::IO(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ParseError> for ErrorKind
{
    fn from(err: ParseError) -> Self
    {
        use ErrorCode::*;

        match err
        {
            ParseError::CorruptStream => CorruptStream.into(),
            ParseError::DuplicateVersion => DuplicateVersion.into(),
            ParseError::DuplicateTagDirective => DuplicateTagDirective.into(),
            ParseError::UndefinedTag => UndefinedTag.into(),
            ParseError::MissingDocumentStart => MissingDocumentStart.into(),
            ParseError::MissingBlockEntry => MissingBlockEntry.into(),
            ParseError::MissingNode => MissingNode.into(),
            ParseError::MissingKey => MissingKey.into(),
            ParseError::MissingFlowSequenceEntryOrEnd => MissingFlowSequenceEntryOrEnd.into(),
            ParseError::MissingFlowMappingEntryOrEnd => MissingFlowMappingEntryOrEnd.into(),
            ParseError::UnexpectedEOF => UnexpectedEOF.into(),
            ParseError::Scanner(e) => ErrorCode::from(e).into(),
            ParseError::UTF8(e) => ErrorKind::Source(e.into()),
            ParseError::IO(e) => ErrorKind::Source(e.into()),
        }
    }
}

impl From<ParseError> for crate::error::Error
{
    fn from(err: ParseError) -> Self
    {
        crate::error::mkError!(err, KIND)
    }
}
