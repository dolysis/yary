/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::node::nodes::{NodeIndex, NodeSpecific};

pub(in crate::node) struct AliasNode
{
    parent: NodeIndex,
    id:     NodeIndex,

    points_to: NodeIndex,
}

impl AliasNode
{
    pub fn new(id: NodeIndex, parent: NodeIndex, points_to: NodeIndex) -> Self
    {
        Self::with_parent(id, parent, points_to)
    }

    pub fn new_with_data(
        parent: NodeIndex,
        points_to: NodeIndex,
    ) -> impl FnOnce(NodeIndex) -> AliasNode
    {
        move |id| Self::new(id, parent, points_to)
    }

    pub fn id(&self) -> NodeIndex
    {
        self.id
    }

    pub fn parent(&self) -> Option<NodeIndex>
    {
        Some(self.parent)
    }

    fn with_parent(id: NodeIndex, parent: NodeIndex, points_to: NodeIndex) -> Self
    {
        Self {
            parent,
            id,
            points_to,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::node) struct AliasData;

impl AliasData
{
    pub fn opaque(self) -> NodeSpecific
    {
        self.into()
    }
}
