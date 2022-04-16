/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::node::{
    error::{NodeError, NodeResult as Result},
    graph::Storage,
    nodes::{
        alias::AliasNode, mapping::MappingNode, scalar::ScalarNode, sequence::SequenceNode, Node,
        NodeData, NodeIndex,
    },
    visitor::{
        transform::{TransformAlias, TransformLeaf, TransformMapping, TransformSequence},
        Context, VisitAlias, VisitLeaf, VisitMapping, VisitSequence, Visitor,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::node) struct GraphVisitor;

impl Visitor for GraphVisitor
{
    fn visit_leaf<'de>(
        &self,
        cxt: &mut Context<'de>,
        graph: &mut Storage<'de>,
        mut node: VisitLeaf<'de>,
    ) -> Result<NodeIndex>
    {
        let (scalar, data) = TransformLeaf::new(cxt, &mut node).consume()?;

        let id = match cxt.current
        {
            Some(parent) =>
            {
                let init = ScalarNode::new_with_data(parent, scalar);

                graph.insert(Node::scalar(init), data)
            },
            None =>
            {
                let init = ScalarNode::root_with_data(scalar);

                graph.insert(Node::scalar(init), data)
            },
        };

        update_context_from_node_data(cxt, &*graph, id)?;
        connect_node_index(cxt, graph, id)?;

        Ok(id)
    }

    fn visit_alias<'de>(
        &self,
        cxt: &mut Context<'de>,
        graph: &mut Storage<'de>,
        mut node: VisitAlias<'de>,
    ) -> Result<NodeIndex>
    {
        let (alias, data) = TransformAlias::new(&*cxt, &mut node).consume()?;

        // Alias nodes *cannot* be root nodes
        let parent = cxt.current.ok_or(NodeError::UndefinedAlias)?;
        let init = AliasNode::new_with_data(parent, alias);

        let id = graph.insert(Node::alias(init), data);

        update_context_from_node_data(cxt, &*graph, id)?;
        connect_node_index(cxt, graph, id)?;

        Ok(id)
    }

    fn visit_sequence<'de>(
        &self,
        cxt: &mut Context<'de>,
        graph: &mut Storage<'de>,
        mut node: VisitSequence<'de>,
    ) -> Result<NodeIndex>
    {
        let data = TransformSequence::new(&*cxt, &mut node).consume()?;

        let id = match cxt.current
        {
            Some(parent) =>
            {
                let init = SequenceNode::new_with(parent);

                graph.insert(Node::sequence(init), data)
            },
            None =>
            {
                let init = SequenceNode::root_with();

                graph.insert(Node::sequence(init), data)
            },
        };

        update_context_from_node_data(cxt, &*graph, id)?;
        connect_node_index(cxt, graph, id)?;

        Ok(id)
    }

    fn visit_mapping<'de>(
        &self,
        cxt: &mut Context<'de>,
        graph: &mut Storage<'de>,
        mut node: VisitMapping<'de>,
    ) -> Result<NodeIndex>
    {
        let data = TransformMapping::new(&*cxt, &mut node).consume()?;

        let id = match cxt.current
        {
            Some(parent) =>
            {
                let init = MappingNode::new_with(parent);

                graph.insert(Node::mapping(init), data)
            },
            None =>
            {
                let init = MappingNode::root_with();

                graph.insert(Node::mapping(init), data)
            },
        };

        update_context_from_node_data(cxt, &*graph, id)?;
        connect_node_index(cxt, graph, id)?;

        Ok(id)
    }
}

/// Update the current context with any additional state
/// that should be stored from the given node_id's data.
///
/// ## Panics
///
/// The given node_id should point to a valid live node
/// in the given Graph. It is considered programmer
/// error to not provide such a node_id, and this
/// function may panic.
fn update_context_from_node_data<'de>(
    cxt: &mut Context<'de>,
    graph: &Storage<'de>,
    node_id: NodeIndex,
) -> Result<()>
{
    let node_data = graph.node_data().get(node_id);
    // It is a logic error to pass a node_id who's data has been
    // deleted
    debug_assert!(node_data.is_some());

    // Add the node's anchor to our context, replacing the
    // existing anchor if one exists.
    if let Some(anchor) = node_data.and_then(NodeData::anchor)
    {
        cxt.aliases.insert(anchor.clone(), node_id);
    }

    Ok(())
}

/// Connect the given node_id into the provided graph,
/// situating it in the current parent node.
///
/// ## Panics
///
/// It is a logic error to call this function if the
/// current parent node is not either a mapping or
/// sequence node, as they are the only kinds of nodes
/// that _can_ have children. Do so will result in a
/// panic, as is considered programmer error.
fn connect_node_index<'de>(
    cxt: &mut Context<'de>,
    graph: &mut Storage<'de>,
    node_id: NodeIndex,
) -> Result<()>
{
    if let Some(parent) = cxt.current
    {
        match graph.nodes_mut()[parent]
        {
            // If we're a child of a mapping node, we need to add ourselves to either the
            // key or value position
            Node::Map(ref mut map) => match cxt.mapping_last_node.take()
            {
                // Found a key stored, so we're a value node, insert ourselves
                Some(key_id) =>
                {
                    map.children_mut().insert(key_id, Some(node_id));
                },
                // No key found, so we're a key node then; insert ourselves into the
                // map, and prime the context for later
                None =>
                {
                    map.children_mut().insert(node_id, None);
                    cxt.mapping_last_node = Some(node_id);
                },
            },
            // Push the node onto the sequence node's children stack
            Node::List(ref mut list) => list.children_mut().push(node_id),
            // Leaf and Alias nodes are never parent nodes
            Node::Leaf(_) | Node::Alias(_) => unreachable!(),
        }
    }

    Ok(())
}
