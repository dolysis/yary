/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::{
    fmt::{self, Debug},
    io,
    str::Utf8Error,
};

use crate::{reader::ReaderError, scanner::error::ScanError};

pub type ParseResult<T> = std::result::Result<T, ParseError>;

#[derive(Debug)]
pub enum ParseError
{
    CorruptStream,
    DuplicateVersion,
    DuplicateTagDirective,
    UndefinedTag,
    MissingDocumentStart,
    MissingBlockEntry,
    MissingNode,
    MissingKey,
    MissingFlowSequenceEntryOrEnd,
    MissingFlowMappingEntryOrEnd,

    Scanner(ScanError),
    UTF8(Utf8Error),
    IO(io::Error),

    UnexpectedEOF,
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
