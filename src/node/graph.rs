/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use slotmap::{SecondaryMap, SlotMap};

use super::nodes::{Node, NodeData, NodeIndex};

struct Graph<'a>
{
    store: Storage<'a>,

    head: Option<NodeIndex>,
}

impl<'a> Graph<'a>
{
    /// Create a new, empty Graph
    pub fn new() -> Self
    {
        Self {
            store: Storage::new(),
            head:  None,
        }
    }

    pub fn insert<F>(&mut self, f: F, data: NodeData<'a>) -> NodeIndex
    where
        F: FnOnce(NodeIndex) -> Node<'a>,
    {
        self.store.insert(f, data)
    }

    pub fn nodes(&self) -> &SlotMap<NodeIndex, Node<'a>>
    {
        self.store.nodes()
    }

    pub fn nodes_mut(&mut self) -> &mut SlotMap<NodeIndex, Node<'a>>
    {
        self.store.nodes_mut()
    }

    pub fn node_data(&self) -> &SecondaryMap<NodeIndex, NodeData<'a>>
    {
        self.store.node_data()
    }

    pub fn node_data_mut(&mut self) -> &mut SecondaryMap<NodeIndex, NodeData<'a>>
    {
        self.store.node_data_mut()
    }
}

#[derive(Default)]
pub(in crate::node) struct Storage<'a>
{
    nodes:     SlotMap<NodeIndex, Node<'a>>,
    node_data: SecondaryMap<NodeIndex, NodeData<'a>>,
}

impl<'a> Storage<'a>
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn nodes(&self) -> &SlotMap<NodeIndex, Node<'a>>
    {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut SlotMap<NodeIndex, Node<'a>>
    {
        &mut self.nodes
    }

    pub fn node_data(&self) -> &SecondaryMap<NodeIndex, NodeData<'a>>
    {
        &self.node_data
    }

    pub fn node_data_mut(&mut self) -> &mut SecondaryMap<NodeIndex, NodeData<'a>>
    {
        &mut self.node_data
    }

    pub fn insert<F>(&mut self, f: F, data: NodeData<'a>) -> NodeIndex
    where
        F: FnOnce(NodeIndex) -> Node<'a>,
    {
        let id = self.nodes_mut().insert_with_key(f);
        self.node_data_mut().insert(id, data);

        id
    }
}
