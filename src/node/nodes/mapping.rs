/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use crate::node::nodes::{NodeIndex, NodeSpecific};

pub(in crate::node) type Children = HashMap<NodeIndex, Option<NodeIndex>>;

pub(in crate::node) struct MappingNode
{
    parent: Option<NodeIndex>,
    id:     NodeIndex,

    children: Children,
}

impl MappingNode
{
    pub fn root(id: NodeIndex) -> Self
    {
        Self::with_parent(id, None)
    }

    pub fn new(id: NodeIndex, parent: NodeIndex) -> Self
    {
        Self::with_parent(id, Some(parent))
    }

    pub fn root_with() -> impl FnOnce(NodeIndex) -> MappingNode
    {
        move |id| Self::root(id)
    }

    pub fn new_with(parent: NodeIndex) -> impl FnOnce(NodeIndex) -> MappingNode
    {
        move |id| Self::new(id, parent)
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
pub(in crate::node) struct MappingData;

impl MappingData
{
    pub fn opaque(self) -> NodeSpecific
    {
        self.into()
    }
}
