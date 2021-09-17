/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

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

    /// A value was expected, but not found
    MissingValue,

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

    /// A plain scalar contained a character sequence that
    /// is not permitted
    InvalidPlainScalar,

    /// A block scalar contained a character sequence that
    /// is not permitted
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

    /// Found a character that cannot start a valid Token
    UnknownDelimiter,

    /// Got end of stream while parsing a token
    UnexpectedEOF,

    /// An integer overflowed
    IntOverflow,

    /// The underlying buffer should be extended before
    /// calling the Scanner again
    Extend,
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
