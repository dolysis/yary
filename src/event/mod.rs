/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! This module exposes methods for directly interacting
//! with YAML event streams.
//!
//! ## Understanding Events
//!
//! Each event produced represents an important semantic
//! change in the underlying YAML byte stream. Broadly,
//! these can be categorized into three spaces:
//!
//! 1. Virtual / Marker
//!     - [`StreamStart`]
//!     - [`StreamEnd`]
//!     - [`DocumentStart`]
//!     - [`DocumentEnd`]
//!
//! 2. Nesting change (+-)
//!     - [`MappingStart`]
//!     - [`MappingEnd`]
//!     - [`SequenceStart`]
//!     - [`SequenceEnd`]
//!
//! 3. Data / Alias
//!     - [`Scalar`]
//!     - [`Alias`]
//!
//! Together, these are used to produce the following
//! productions:
//!
//! ```text
//! stream          := StreamStart document+ StreamEnd
//! document        := DocumentStart content? DocumentEnd
//! content         := Scalar | collection
//! collection      := sequence | mapping
//! sequence        := SequenceStart node* SequenceEnd
//! mapping         := MappingStart (node node)* MappingEnd
//! node            := Alias | content
//!
//! ?               => 0 or 1 of prefix
//! *               => 0 or more of prefix
//! +               => 1 or more of prefix
//! ()              => production grouping
//! |               => production logical OR
//! ```
//!
//! In addition to the various [`Event`] types, every
//! [`Node`] also provides a hint as to its placement in the
//! stream via its [`NodeKind`]. Together, these should
//! allow users to maintain relatively little external state
//! regarding the [`Event`] stream, beyond anything they
//! wish to collect from the stream.
//!
//! [`StreamStart`]:    enum@types::EventData::StreamStart
//! [`StreamEnd`]:      enum@types::EventData::StreamEnd
//! [`DocumentStart`]:  enum@types::EventData::DocumentStart
//! [`DocumentEnd`]:    enum@types::EventData::DocumentEnd
//! [`MappingStart`]:   enum@types::EventData::MappingStart
//! [`MappingEnd`]:     enum@types::EventData::MappingEnd
//! [`SequenceStart`]:  enum@types::EventData::SequenceStart
//! [`SequenceEnd`]:    enum@types::EventData::SequenceEnd
//! [`Scalar`]:         enum@types::EventData::Scalar
//! [`Alias`]:          enum@types::EventData::Alias
//! [`Node`]:           struct@types::Node
//! [`NodeKind`]:       enum@types::NodeKind
//! [`Token`]:          enum@crate::token::Token
//! [`Read`]:           trait@crate::reader::Read

mod parser;
mod state;

pub mod error;
pub mod types;
