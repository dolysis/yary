/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use slotmap::new_key_type;

use crate::node::{
    nodes::{
        alias::{AliasData, AliasNode},
        mapping::{MappingData, MappingNode},
        scalar::{ScalarData, ScalarNode},
        sequence::{SequenceData, SequenceNode},
    },
    Slice,
};

pub(in crate::node) mod alias;
pub(in crate::node) mod mapping;
pub(in crate::node) mod scalar;
pub(in crate::node) mod sequence;

new_key_type! {
    /// Identifier used for locating [Node]s in a graph.
    ///
    /// An invariant of this type is that one should never use
    /// [NodeIndex]s as indexes into graphs that did not generate
    /// the [NodeIndex]. Behavior is safe and cannot cause UB,
    /// but is unspecified, and never what you want.
    pub(in crate::node) struct NodeIndex;
}

/// Possible nodes one can encounter while traversing a
/// graph.
///
/// Importantly, only `LeafNode`s contain data, all other
/// variants deal with the structure of the graph and can be
/// considered internal nodes.
pub(in crate::node) enum Node<'de>
{
    /// Data container, storing a single scalar node
    Leaf(ScalarNode<'de>),
    /// Mapping node, storing key value node pairs
    Map(MappingNode),
    /// List node, storing a sequence of nodes
    List(SequenceNode),
    /// Alias node pointing to a previously seen node
    Alias(AliasNode),
}

impl<'de> Node<'de>
{
    /// Wrap the new scalar node produced by f
    pub fn scalar<F>(f: F) -> impl FnOnce(NodeIndex) -> Node<'de>
    where
        F: FnOnce(NodeIndex) -> ScalarNode<'de>,
    {
        |id| Node::Leaf(f(id))
    }

    /// Wrap the new mapping node produced by f
    pub fn mapping<F>(f: F) -> impl FnOnce(NodeIndex) -> Node<'de>
    where
        F: FnOnce(NodeIndex) -> MappingNode,
    {
        |id| Node::Map(f(id))
    }

    /// Wrap the new sequence node produced by f
    pub fn sequence<F>(f: F) -> impl FnOnce(NodeIndex) -> Node<'de>
    where
        F: FnOnce(NodeIndex) -> SequenceNode,
    {
        |id| Node::List(f(id))
    }

    /// Wrap the new alias node produced by f
    pub fn alias<F>(f: F) -> impl FnOnce(NodeIndex) -> Node<'de>
    where
        F: FnOnce(NodeIndex) -> AliasNode,
    {
        |id| Node::Alias(f(id))
    }
}

#[derive(Debug, Clone)]
pub(in crate::node) struct NodeData<'de>
{
    anchor:  Option<Slice<'de>>,
    tag:     Option<Tag<'de>>,
    context: NodeContext,
    mark:    NodeMark,

    node_specific: NodeSpecific,
}

impl<'de> NodeData<'de>
{
    pub fn new(
        anchor: Option<Slice<'de>>,
        tag: Option<Tag<'de>>,
        context: NodeContext,
        mark: NodeMark,
        ns: NodeSpecific,
    ) -> Self
    {
        Self {
            anchor,
            tag,
            context,
            mark,
            node_specific: ns,
        }
    }

    pub fn anchor(&self) -> Option<&Slice<'de>>
    {
        self.anchor.as_ref()
    }

    pub fn tag(&self) -> Option<&Tag<'de>>
    {
        self.tag.as_ref()
    }

    pub fn context(&self) -> NodeContext
    {
        self.context
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node) struct Tag<'de>
{
    handle: Slice<'de>,
    prefix: Slice<'de>,
    suffix: Slice<'de>,
}

impl<'de> Tag<'de>
{
    pub fn new(handle: Slice<'de>, prefix: Slice<'de>, suffix: Slice<'de>) -> Self
    {
        Self {
            handle,
            prefix,
            suffix,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::node) enum NodeContext
{
    Block,
    Flow,
}

impl NodeContext
{
    pub const fn is_block(self) -> bool
    {
        matches!(self, Self::Block)
    }

    pub const fn is_flow(self) -> bool
    {
        !self.is_block()
    }
}

impl Default for NodeContext
{
    fn default() -> Self
    {
        Self::Block
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::node) struct NodeMark
{
    start: usize,
    end:   usize,
}

impl NodeMark
{
    pub const fn new(start: usize, end: usize) -> Self
    {
        Self { start, end }
    }

    pub const fn start(&self) -> usize
    {
        self.start
    }

    pub const fn end(&self) -> usize
    {
        self.end
    }
}

impl From<(usize, usize)> for NodeMark
{
    fn from((start, end): (usize, usize)) -> Self
    {
        Self::new(start, end)
    }
}

#[derive(Debug, Clone)]
pub(in crate::node) enum NodeSpecific
{
    Mapping(MappingData),
    Sequence(SequenceData),
    Alias(AliasData),
    Scalar(ScalarData),
}

impl From<ScalarData> for NodeSpecific
{
    fn from(data: ScalarData) -> Self
    {
        Self::Scalar(data)
    }
}

impl From<AliasData> for NodeSpecific
{
    fn from(data: AliasData) -> Self
    {
        Self::Alias(data)
    }
}

impl From<MappingData> for NodeSpecific
{
    fn from(data: MappingData) -> Self
    {
        Self::Mapping(data)
    }
}

impl From<SequenceData> for NodeSpecific
{
    fn from(data: SequenceData) -> Self
    {
        Self::Sequence(data)
    }
}
