use std::fmt;

pub type ScanResult<T> = std::result::Result<T, ScanError>;

#[derive(Debug, PartialEq, Eq)]
pub enum ScanError
{
    /// Directive was not either YAML or TAG
    UnknownDirective,

    /// %YAML 1.1
    ///       ^
    MissingMajor,

    /// %YAML 1.1
    ///         ^
    MissingMinor,

    /// A directive major or minor digit was not 0..=9
    InvalidVersion,

    /// Tag handle was not primary (!), secondary (!!) or
    /// named (!alphanumeric!)
    InvalidTagHandle,

    /// Tag prefix was not separated from the handle by one
    /// or more spaces
    InvalidTagPrefix,

    /// Tag suffix was invalid
    InvalidTagSuffix,

    /// Either an anchor (*) or alias (&)'s name was invalid
    InvalidAnchorName,

    /// A flow scalar was invalid for some reason
    InvalidFlowScalar,

    /// A character that was not valid for the escape
    /// sequence was encountered
    UnknownEscape,

    /// Got end of stream while parsing a token
    UnexpectedEOF,
}

impl fmt::Display for ScanError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        // Delegate to debug for the moment
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for ScanError {}
