/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! This library exposes methods for interacting with YAML
//! byte streams.
//!
//! It is currently still in development, and will likely
//! have multiple breaking changes to the exposed API before
//! stabilizing. Use at your own risk.
//!
//! The exposed APIs are grouped by module, and no high
//! level API yet exists for this library, though this will
//! change in the future.

#![allow(dead_code)]
#![allow(clippy::suspicious_else_formatting)]

pub mod event;
pub mod reader;

mod error;
mod queue;
mod scanner;
mod token;
