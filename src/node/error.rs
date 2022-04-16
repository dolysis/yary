/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! This module contains the errors that may surface while
//! parsing a YAML event stream into memory.

use std::fmt::{self, Debug};

use crate::{
    error::{
        internal::{ErrorCode, ErrorKind},
        mkError,
    },
    event::error::ParseError,
    scanner::error::ScanError,
};

/// Result type returned by [`yary::node`](super)
pub(crate) type NodeResult<T> = std::result::Result<T, NodeError>;

/// Possible errors that can be encountered while parsing
/// YAML graph structures.
#[derive(Debug)]
pub(crate) enum NodeError
{
    UndefinedAlias,

    Parser(ParseError),
    Scanner(ScanError),
}

impl From<ParseError> for NodeError
{
    fn from(err: ParseError) -> Self
    {
        Self::Parser(err)
    }
}

impl From<ScanError> for NodeError
{
    fn from(err: ScanError) -> Self
    {
        Self::Scanner(err)
    }
}

impl fmt::Display for NodeError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for NodeError
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>
    {
        match self
        {
            Self::Parser(e) => Some(e),
            _ => None,
        }
    }
}

impl From<NodeError> for ErrorKind
{
    fn from(err: NodeError) -> Self
    {
        use ErrorCode::*;

        match err
        {
            NodeError::UndefinedAlias => UndefinedAlias.into(),
            NodeError::Parser(e) => e.into(),
            NodeError::Scanner(e) => ErrorCode::from(e).into(),
        }
    }
}

impl From<NodeError> for crate::error::Error
{
    fn from(err: NodeError) -> Self
    {
        mkError!(err, KIND)
    }
}
