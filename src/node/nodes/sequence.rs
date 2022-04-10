/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::node::nodes::{NodeIndex, NodeSpecific};

pub(in crate::node) type Children = Vec<NodeIndex>;

pub(in crate::node) struct SequenceNode
{
    parent: Option<NodeIndex>,
    id:     NodeIndex,

    children: Children,
}

impl SequenceNode
{
    pub fn root(id: NodeIndex) -> Self
    {
        Self::with_parent(id, None)
    }

    pub fn new(id: NodeIndex, parent: NodeIndex) -> Self
    {
        Self::with_parent(id, Some(parent))
    }

    pub fn root_with() -> impl FnOnce(NodeIndex) -> SequenceNode
    {
        move |id| Self::root(id)
    }

    pub fn new_with(parent: NodeIndex) -> impl FnOnce(NodeIndex) -> SequenceNode
    {
        move |id| Self::new(id, parent)
    }

    pub fn id(&self) -> NodeIndex
    {
        self.id
    }

    pub fn parent(&self) -> Option<NodeIndex>
    {
        self.parent
    }

    pub fn children(&self) -> &Children
    {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut Children
    {
        &mut self.children
    }

    fn with_parent(id: NodeIndex, parent: Option<NodeIndex>) -> Self
    {
        Self {
            parent,
            id,
            children: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::node) struct SequenceData;

impl SequenceData
{
    pub fn opaque(self) -> NodeSpecific
    {
        self.into()
    }
}
