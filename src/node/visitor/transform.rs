/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! This module contains structures for transforming various
//! visitable node types into formats that our Graph
//! representation can consume

use std::mem::take;

use crate::{
    event::types::Scalar,
    node::{
        error::{NodeError, NodeResult as Result},
        nodes::{
            alias::AliasData, mapping::MappingData, scalar::ScalarData, sequence::SequenceData,
            NodeContext, NodeData, NodeIndex, Tag,
        },
        visitor::{Context, VisitAlias, VisitLeaf, VisitMapping, VisitSequence},
    },
    token::Slice,
};

/// Transforms a visited leaf node into a (scalar, data)
/// tuple, which can mapped into a graph structure
pub(super) struct TransformLeaf<'a, 'b, 'de>
{
    cxt:  &'a Context<'de>,
    node: &'b mut VisitLeaf<'de>,
}

impl<'a, 'b, 'de> TransformLeaf<'a, 'b, 'de>
{
    /// Instantiate a new [TransformLeaf] for the given
    /// context and node.
    pub fn new(cxt: &'a Context<'de>, node: &'b mut VisitLeaf<'de>) -> Self
    {
        Self { cxt, node }
    }

    /// Consume the node, returning a (scalar, data) tuple
    ///
    /// This operation is destructive. Running it more than
    /// once on a single node may return faulty data or
    /// panic.
    pub fn consume(mut self) -> Result<(Slice<'de>, NodeData<'de>)>
    {
        let mut scalar = self.take_scalar()?;
        let data = self.take_data(&scalar);
        let slice = take(scalar.data_mut());

        Ok((slice, data))
    }

    /// Retrieves the scalar part of the node.
    ///
    /// This function is only safe to call once, future
    /// calls will return unspecified results.
    fn take_scalar(&mut self) -> Result<Scalar<'de>>
    {
        take(&mut self.node.node.content)
            .evaluate_scalar()
            .map_err(Into::into)
    }

    /// Retrieves the data part of the node.
    ///
    /// This function is only safe to call once, future
    /// calls will return unspecified results.
    fn take_data(&mut self, scalar: &Scalar<'de>) -> NodeData<'de>
    {
        let mark = (self.node.start, self.node.end).into();
        let ns = ScalarData::new(scalar.style()).into();
        let tag = new_tag_from_context(&*self.cxt, take(&mut self.node.node.tag));
        let anchor = take(&mut self.node.node.anchor);
        let context = new_node_context();

        NodeData::new(anchor, tag, context, mark, ns)
    }
}

/// Transforms a visited alias node returning an (alias,
/// data) tuple, which can mapped into a graph structure
pub(super) struct TransformAlias<'a, 'b, 'de>
{
    cxt:  &'a Context<'de>,
    node: &'b mut VisitAlias<'de>,
}

impl<'a, 'b, 'de> TransformAlias<'a, 'b, 'de>
{
    /// Instantiate a new [TransformAlias] for the given
    /// context and node.
    pub fn new(cxt: &'a Context<'de>, node: &'b mut VisitAlias<'de>) -> Self
    {
        Self { cxt, node }
    }

    /// Consume the node, returning a (alias, data) tuple
    ///
    /// This operation is destructive. Running it more than
    /// once on a single node may return faulty data or
    /// panic.
    pub fn consume(mut self) -> Result<(NodeIndex, NodeData<'de>)>
    {
        let alias = self.deference_alias()?;
        let data = self.take_data();

        Ok((alias, data))
    }

    /// Retrieve the aliased node by name from the current
    /// context, returning an error if the lookup failed.
    fn deference_alias(&mut self) -> Result<NodeIndex>
    {
        self.cxt
            .aliases
            .get(&self.node.alias)
            .copied()
            .ok_or(NodeError::UndefinedAlias)
    }

    /// Retrieves the data part of the node.
    ///
    /// This function is only safe to call once, future
    /// calls will return unspecified results.
    fn take_data(&mut self) -> NodeData<'de>
    {
        let mark = (self.node.start, self.node.end).into();
        let ns = AliasData.into();
        let context = new_node_context();

        NodeData::new(None, None, context, mark, ns)
    }
}

/// Transforms a visited sequence node returning the data
/// which can mapped into a graph structure
pub(super) struct TransformSequence<'a, 'b, 'de>
{
    cxt:  &'a Context<'de>,
    node: &'b mut VisitSequence<'de>,
}

impl<'a, 'b, 'de> TransformSequence<'a, 'b, 'de>
{
    /// Instantiate a new [TransformSequence] for the given
    /// context and node.
    pub fn new(cxt: &'a Context<'de>, node: &'b mut VisitSequence<'de>) -> Self
    {
        Self { cxt, node }
    }

    /// Consume the node, returning the data attached
    ///
    /// This operation is destructive. Running it more than
    /// once on a single node may return faulty data or
    /// panic.
    pub fn consume(mut self) -> Result<NodeData<'de>>
    {
        Ok(self.take_data())
    }

    /// Retrieves the data part of the node.
    ///
    /// This function is only safe to call once, future
    /// calls will return unspecified results.
    fn take_data(&mut self) -> NodeData<'de>
    {
        let mark = (self.node.start, self.node.end).into();
        let tag = new_tag_from_context(&*self.cxt, take(&mut self.node.node.tag));
        let anchor = take(&mut self.node.node.anchor);
        let context = new_node_context();

        NodeData::new(anchor, tag, context, mark, SequenceData.into())
    }
}

/// Transforms a visited mapping node returning the data
/// which can mapped into a graph structure
pub(super) struct TransformMapping<'a, 'b, 'de>
{
    cxt:  &'a Context<'de>,
    node: &'b mut VisitMapping<'de>,
}

impl<'a, 'b, 'de> TransformMapping<'a, 'b, 'de>
{
    /// Instantiate a new [TransformMapping] for the given
    /// context and node.
    pub fn new(cxt: &'a Context<'de>, node: &'b mut VisitMapping<'de>) -> Self
    {
        Self { cxt, node }
    }

    /// Consume the node, returning the data attached
    ///
    /// This operation is destructive. Running it more than
    /// once on a single node may return faulty data or
    /// panic.
    pub fn consume(mut self) -> Result<NodeData<'de>>
    {
        Ok(self.take_data())
    }

    /// Retrieves the data part of the node.
    ///
    /// This function is only safe to call once, future
    /// calls will return unspecified results.
    fn take_data(&mut self) -> NodeData<'de>
    {
        let mark = (self.node.start, self.node.end).into();
        let tag = new_tag_from_context(&*self.cxt, take(&mut self.node.node.tag));
        let anchor = take(&mut self.node.node.anchor);
        let context = new_node_context();

        NodeData::new(anchor, tag, context, mark, MappingData.into())
    }
}

/// Generate a [Tag] from the given (handle, suffix) tuple,
/// looking up the prefix from the current context; if the
/// tuple is some.
fn new_tag_from_context<'de>(
    cxt: &Context<'de>,
    tag: Option<(Slice<'de>, Slice<'de>)>,
) -> Option<Tag<'de>>
{
    let tags = &cxt.tag_directives;

    let (handle, suffix) = tag?;
    let prefix = tags.get(&handle)?.clone();

    Tag::new(handle, prefix, suffix).into()
}

// TODO: Actually return the node context once it is present
// in the EventData
/// Placeholder function for retrieving a node's context
fn new_node_context() -> NodeContext
{
    NodeContext::Block
}
