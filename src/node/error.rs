/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! This module contains the errors that may surface while
//! parsing a YAML event stream into memory.

use crate::{event::error::ParseError, scanner::error::ScanError};

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
