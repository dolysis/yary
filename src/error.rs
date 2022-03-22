/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{error::Error as StdError, fmt};

/// Result typedef used throughout this library's public API
pub type Result<T> = std::result::Result<T, Error>;

/// Opaque type representing all possible errors which can
/// occur during library usage.
pub struct Error
{
    inner: Box<internal::Error>,
}

impl Error
{
    /// The line at which the error occurred.
    pub fn line(&self) -> u64
    {
        self.inner.line
    }

    /// The column into the line where the error occurred.
    pub fn column(&self) -> u64
    {
        self.inner.column
    }

    /// The index into the byte stream at which the error
    /// occurred.
    pub fn at(&self) -> u64
    {
        self.inner.at
    }

    /// Categorize the error into one of the following:
    ///
    /// - [`Category::Syntax`] The YAML stream was
    ///   syntactically invalid
    /// - [`Category::Data`] The YAML stream contained data
    ///   that could not be parsed
    /// - [`Category::IO`] The underlying byte stream
    ///   surfaced an error while doing IO
    /// - [`Category::EOF`] The YAML stream ended
    ///   unexpectedly
    pub fn classify(&self) -> Category
    {
        self.inner.classify()
    }

    /// Checks whether this error was contextualized.
    ///
    /// If this method returns false then the methods
    ///
    /// - [`at()`](#method.at)
    /// - [`column()`](#method.column)
    /// - [`line()`](#method.line)
    ///
    /// will return meaningless values.
    pub fn has_context(&self) -> bool
    {
        self.inner.has_context()
    }

    /// Boxes the internal error, returning new public error
    /// type
    pub(crate) fn new(err: internal::Error) -> Self
    {
        Self {
            inner: Box::new(err),
        }
    }
}

/// Rough category of an [`Error`].
///
/// Useful for making decisions upon encountering an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category
{
    /// The underlying byte stream returned an error while
    /// attempting IO
    IO,

    /// The YAML stream was not syntactically valid
    Syntax,

    /// There was an issue with the data contained in the
    /// YAML stream (e.g: an integer overflowed)
    Data,

    /// The YAML stream unexpectedly terminated before it
    /// was appropriate to do so
    EOF,
}

pub(crate) mod internal
{
    use std::{error::Error as StdError, fmt, io, str::Utf8Error};

    use super::Category;

    /// Internal error representation used throughout the
    /// library.
    ///
    /// Contains enough metadata about the position of the
    /// error that we can contextualize it later
    pub(crate) struct Error
    {
        /// Error variant encountered
        pub kind:   ErrorKind,
        /// Position in the byte stream that the error
        /// occurred
        pub at:     u64,
        /// Line number of the error
        pub line:   u64,
        /// Offset into current line the error occurred at
        pub column: u64,
    }

    impl Error
    {
        /// Create a new, contextless [`Error`].
        ///
        /// Care should be taken to later apply context, if
        /// at all possible.
        pub fn new<T>(err: T) -> Self
        where
            T: Into<ErrorKind>,
        {
            Self::with_context(err, 0, 0, 0)
        }

        /// Create a new, contextualized [`Error`].
        pub fn with_context<T>(err: T, at: u64, line: u64, column: u64) -> Self
        where
            T: Into<ErrorKind>,
        {
            let kind = err.into();

            Self {
                kind,
                at,
                line,
                column,
            }
        }

        /// Consume self by value while applying a mutating
        /// closure to `self`
        pub fn with<F>(self, f: F) -> Self
        where
            F: FnOnce(&mut Self) -> &mut Self,
        {
            let mut this = self;

            f(&mut this);

            this
        }

        /// Add context to this error, replacing the
        /// existing context (if any exists).
        pub fn context(&mut self, at: u64, line: u64, column: u64) -> &mut Self
        {
            self.at = at;
            self.line = line;
            self.column = column;

            self
        }

        /// Replace the error cause with the given kind.
        pub fn kind<T>(&mut self, kind: T) -> &mut Self
        where
            T: Into<ErrorKind>,
        {
            self.kind = kind.into();

            self
        }

        /// Checks whether this error is contextualized
        pub fn has_context(&self) -> bool
        {
            // Only errors created without context will have a line
            // number of 0
            self.line != 0
        }

        /// Categorize this error
        pub fn classify(&self) -> Category
        {
            Into::into(&self.kind)
        }
    }

    /// Unified wrapper around the actual error variants we
    /// can produce
    #[derive(Debug)]
    pub(crate) enum ErrorKind
    {
        Code(ErrorCode),
        Source(SourceError),
    }

    /// Lightweight errors, specific to this library.
    ///
    /// This enum should never be polluted with large
    /// variants, or wrap underlying errors. Use
    /// [`SourceError`] for those.
    ///
    /// Library hot-paths should be able to return this
    /// without adversely affecting the speed of unwinding
    /// up the stack, and while we do not force the
    /// compiler, we would prefer that:
    ///
    /// `size_of::<Self> == size_of::<u8>`
    ///
    /// is true.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(crate) enum ErrorCode
    {
        /*
         * ==== Scanner Errors ====
         */
        /// Got end of stream while parsing a token
        UnexpectedEOF,

        /// Directive was not either YAML or TAG
        UnknownDirective,

        /// %YAML 1.1
        ///       ^
        MissingMajor,

        /// %YAML 1.1
        ///         ^
        MissingMinor,

        /// A value was expected, but not found
        MissingValue,

        /// A directive major or minor digit was not 0..=9
        InvalidVersion,

        /// Tag handle was not primary (!), secondary (!!)
        /// or named (!alphanumeric!)
        InvalidTagHandle,

        /// Tag prefix was not separated from the handle by
        /// one or more spaces
        InvalidTagPrefix,

        /// Tag suffix was invalid
        InvalidTagSuffix,

        /// Either an anchor (*) or alias (&)'s name was
        /// invalid
        InvalidAnchorName,

        /// A flow scalar was invalid for some reason
        InvalidFlowScalar,

        /// A plain scalar contained a character sequence
        /// that is not permitted
        InvalidPlainScalar,

        /// A block scalar contained a character sequence
        /// that is not permitted
        InvalidBlockScalar,

        /// A block entry was not expected or allowed
        InvalidBlockEntry,

        /// A tab character '\t' was found in an invalid
        /// context, typically block indentation
        InvalidTab,

        /// A mapping key was not expected or allowed
        InvalidKey,

        /// A mapping value was not expected or allowed
        InvalidValue,

        /// A character that was not valid for the escape
        /// sequence was encountered
        UnknownEscape,

        /// Found a character that cannot start a valid
        /// Token
        UnknownDelimiter,

        /// An integer overflowed
        IntOverflow,

        /*
         * ==== Parser Errors ====
         */
        /// A mismatch between the stream's actual state and
        /// what the parser is expecting occurred.
        ///
        /// Typically, this only happens if a [`Read`]
        /// source is given to two separate parsers.
        ///
        /// [`Read`]: trait@crate::reader::Read
        CorruptStream,

        /// More than one `%YAML` directive was found inside
        /// a single document's context.
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

        /// More than one `%TAG` directive was found _for
        /// the same handle_ inside a single
        /// document's context.
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

        /// A tag referenced a handle that has not been
        /// defined.
        ///
        /// ## Examples
        ///
        /// ```yaml
        /// ---
        /// key: !handle! value
        /// #    ^^^^^^^^ UndefinedTag
        /// ```
        UndefinedTag,

        /// In certain cases YAML requires an indication
        /// that another document is being started,
        /// necessitating a DocumentStart '---'
        /// symbol.
        ///
        /// Most commonly, if a stream contains two
        /// documents the first must include a
        /// DocumentEnd ('...') symbol,
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
        /// YAML tends to be very forgiving with missing
        /// nodes, but it is possible to construct a
        /// YAML stream with a required Node, most
        /// commonly if using tags or anchors.
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

        /// A YAML flow sequence was missing a terminus
        /// (']') or continuation (',') symbol.
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

        /// A YAML flow mapping was missing a terminus ('}')
        /// or continuation (',') symbol.
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
    }

    /// Heavy and/or external errors that can occur during
    /// library usage
    #[derive(Debug)]
    pub(crate) enum SourceError
    {
        /*
         * ==== Reader Errors ====
         */
        /// Catch all wrapper for any underlying IO errors
        /// reported to us
        IO(io::Error),

        /// Encountered invalid an UTF8 sequence
        UTF8(Utf8Error),
    }

    impl fmt::Debug for Error
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
        {
            let mut this = f.debug_struct("Error");

            this.field("kind", &self.kind);

            if self.has_context()
            {
                this.field("line", &self.line)
                    .field("column", &self.column)
                    .field("index", &self.at);
            }

            this.finish()
        }
    }

    impl fmt::Display for Error
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
        {
            use fmt::Display;

            if self.has_context()
            {
                f.write_fmt(format_args!(
                    "{}, on line {}, column {}, at index {}",
                    self.kind, self.line, self.column, self.at
                ))
            }
            else
            {
                Display::fmt(&self.kind, f)
            }
        }
    }

    impl StdError for Error
    {
        fn source(&self) -> Option<&(dyn StdError + 'static)>
        {
            StdError::source(&self.kind)
        }
    }

    impl From<Error> for super::Error
    {
        fn from(err: Error) -> Self
        {
            Self::new(err)
        }
    }

    impl<T> From<T> for Error
    where
        T: Into<ErrorKind>,
    {
        fn from(t: T) -> Self
        {
            Error::new(t.into())
        }
    }

    impl From<Error> for std::io::Error
    {
        fn from(err: Error) -> Self
        {
            if let ErrorKind::Source(SourceError::IO(err)) = err.kind
            {
                return err;
            }

            match err.classify()
            {
                Category::Syntax => io::Error::new(io::ErrorKind::InvalidInput, err),
                Category::Data => io::Error::new(io::ErrorKind::InvalidData, err),
                Category::EOF => io::Error::new(io::ErrorKind::UnexpectedEof, err),
                Category::IO => unreachable!(),
            }
        }
    }

    impl From<&'_ ErrorKind> for Category
    {
        fn from(kind: &'_ ErrorKind) -> Self
        {
            match kind
            {
                ErrorKind::Code(e) => e.into(),
                ErrorKind::Source(e) => e.into(),
            }
        }
    }

    impl fmt::Display for ErrorKind
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
        {
            match self
            {
                ErrorKind::Code(ref e) => fmt::Display::fmt(e, f),
                ErrorKind::Source(ref e) => fmt::Display::fmt(e, f),
            }
        }
    }

    impl StdError for ErrorKind {}

    impl From<ErrorCode> for ErrorKind
    {
        fn from(e: ErrorCode) -> Self
        {
            ErrorKind::Code(e)
        }
    }

    impl From<SourceError> for ErrorKind
    {
        fn from(e: SourceError) -> Self
        {
            ErrorKind::Source(e)
        }
    }

    impl From<ErrorCode> for Category
    {
        fn from(code: ErrorCode) -> Self
        {
            use ErrorCode::*;

            match code
            {
                UnknownDirective
                | MissingMajor
                | MissingMinor
                | MissingValue
                | InvalidVersion
                | InvalidTagHandle
                | InvalidTagPrefix
                | InvalidTagSuffix
                | InvalidAnchorName
                | InvalidFlowScalar
                | InvalidPlainScalar
                | InvalidBlockScalar
                | InvalidBlockEntry
                | InvalidTab
                | InvalidKey
                | InvalidValue
                | UnknownEscape
                | UnknownDelimiter
                | DuplicateVersion
                | DuplicateTagDirective
                | UndefinedTag
                | MissingDocumentStart
                | MissingBlockEntry
                | MissingNode
                | MissingKey
                | MissingFlowSequenceEntryOrEnd
                | MissingFlowMappingEntryOrEnd => Category::Syntax,

                IntOverflow | CorruptStream => Category::Data,

                UnexpectedEOF => Category::EOF,
            }
        }
    }

    impl From<&'_ ErrorCode> for Category
    {
        fn from(code: &'_ ErrorCode) -> Self
        {
            From::from(*code)
        }
    }

    impl fmt::Display for ErrorCode
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
        {
            use ErrorCode::*;

            match self
            {
                UnexpectedEOF => f.write_str("unexpected EOF"),
                UnknownDirective => f.write_str("unknown YAML directive"),
                MissingMajor => f.write_str("%YAML directive missing major version"),
                MissingMinor => f.write_str("%YAML directive missing minor version"),
                MissingValue => f.write_str("missing YAML mapping value"),
                InvalidVersion => f.write_str("%YAML directive is invalid"),
                InvalidTagHandle =>
                {
                    f.write_str("node tag handle is not one of !, !!, or ![a-zA-Z0-9]+!")
                },
                InvalidTagPrefix => f.write_str("%TAG directive prefix is invalid"),
                InvalidTagSuffix => f.write_str("node tag suffix is invalid"),
                InvalidAnchorName =>
                {
                    f.write_str("node anchor or alias contains invalid characters")
                },
                InvalidFlowScalar => f.write_str("flow scalar contains invalid characters"),
                InvalidPlainScalar => f.write_str("plain scalar contains invalid characters"),
                InvalidBlockScalar => f.write_str("block scalar contains invalid characters"),
                InvalidBlockEntry => f.write_str("block entry is not allowed in this context"),
                InvalidTab => f.write_str("indentation contained a tab character"),
                InvalidKey => f.write_str("key node is not allowed in this context"),
                InvalidValue => f.write_str("value node is not allowed in this context"),
                UnknownEscape => f.write_str("unknown escape sequence"),
                UnknownDelimiter => f.write_str("unknown token"),
                IntOverflow => f.write_str("integer overflow while parsing"),
                CorruptStream => f.write_str("invalid or corrupt yaml stream"),
                DuplicateVersion => f.write_str("duplicate %YAML directive found in a document"),
                DuplicateTagDirective =>
                {
                    f.write_str("duplicate %TAG directive for a handle found in a document")
                },
                UndefinedTag => f.write_str("undefined node tag found"),
                MissingDocumentStart => f.write_str("missing document start or end indicator"),
                MissingBlockEntry => f.write_str("block entry was expected"),
                MissingNode => f.write_str("node was expected"),
                MissingKey => f.write_str("key node was expected"),
                MissingFlowSequenceEntryOrEnd =>
                {
                    f.write_str("missing flow sequence delimiter ',' or ']'")
                },
                MissingFlowMappingEntryOrEnd =>
                {
                    f.write_str("missing flow mapping delimiter ',' or '}'")
                },
            }
        }
    }

    impl StdError for ErrorCode {}

    impl From<&'_ SourceError> for Category
    {
        fn from(err: &'_ SourceError) -> Self
        {
            match err
            {
                SourceError::IO(_) => Category::IO,
                SourceError::UTF8(_) => Category::Data,
            }
        }
    }

    impl fmt::Display for SourceError
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
        {
            match self
            {
                SourceError::IO(ref e) => fmt::Display::fmt(e, f),
                SourceError::UTF8(ref e) => fmt::Display::fmt(e, f),
            }
        }
    }

    impl StdError for SourceError
    {
        fn source(&self) -> Option<&(dyn StdError + 'static)>
        {
            match self
            {
                SourceError::IO(e) => Some(e),
                SourceError::UTF8(e) => Some(e),
            }
        }
    }

    impl From<Utf8Error> for SourceError
    {
        fn from(e: Utf8Error) -> Self
        {
            SourceError::UTF8(e)
        }
    }

    impl From<io::Error> for SourceError
    {
        fn from(e: io::Error) -> Self
        {
            SourceError::IO(e)
        }
    }
}

impl fmt::Debug for Error
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl fmt::Display for Error
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl StdError for Error
{
    fn source(&self) -> Option<&(dyn StdError + 'static)>
    {
        StdError::source(&self.inner)
    }
}

impl From<Error> for std::io::Error
{
    fn from(err: Error) -> Self
    {
        From::from(*err.inner)
    }
}
