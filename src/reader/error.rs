/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Error types returned from the [`yary::reader`](super)
//! module.

use std::{error::Error as StdError, fmt, io, str::Utf8Error};

use crate::{
    error::internal::{ErrorCode, ErrorKind, SourceError},
    scanner::error::ScanError,
};

/// Type alias of the `Result`s returned from this module
pub type ReaderResult<T> = std::result::Result<T, ReaderError>;

/// Possible errors that can occur while reading from YAML
/// byte streams
#[derive(Debug)]
pub enum ReaderError
{
    /// Encountered invalid an UTF8 sequence
    UTF8(Utf8Error),
    /// Catch all wrapper for any underlying IO errors
    /// reported to us
    IO(io::Error),
    Scanner(ScanError),
}

impl fmt::Display for ReaderError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Debug::fmt(&self, f)
    }
}

impl StdError for ReaderError
{
    fn source(&self) -> Option<&(dyn StdError + 'static)>
    {
        match self
        {
            ReaderError::UTF8(ref e) => Some(e),
            ReaderError::IO(ref e) => Some(e),
            ReaderError::Scanner(ref e) => Some(e),
        }
    }
}

impl From<Utf8Error> for ReaderError
{
    fn from(e: Utf8Error) -> Self
    {
        Self::UTF8(e)
    }
}

impl From<io::Error> for ReaderError
{
    fn from(e: io::Error) -> Self
    {
        Self::IO(e)
    }
}

impl From<ScanError> for ReaderError
{
    fn from(e: ScanError) -> Self
    {
        Self::Scanner(e)
    }
}

impl From<ReaderError> for ErrorKind
{
    fn from(err: ReaderError) -> Self
    {
        match err
        {
            ReaderError::UTF8(e) => SourceError::UTF8(e).into(),
            ReaderError::IO(e) => SourceError::IO(e).into(),
            ReaderError::Scanner(e) => ErrorCode::from(e).into(),
        }
    }
}
