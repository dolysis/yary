use std::fmt;

pub type ScanResult<T> = std::result::Result<T, ScanError>;

#[derive(Debug)]
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
