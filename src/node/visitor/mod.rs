/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use crate::{
    event::types::{TagDirectives, VersionDirective, DEFAULT_VERSION},
    node::{error::NodeResult as Result, graph::Storage, nodes::NodeIndex},
    token::Slice,
};

mod transform;

pub(in crate::node) trait Visitor
{
    fn visit_leaf<'de>(
        &self,
        cxt: &mut Context<'de>,
        graph: &mut Storage<'de>,
        node: VisitLeaf<'de>,
    ) -> Result<NodeIndex>;

    fn visit_alias<'de>(
        &self,
        cxt: &mut Context<'de>,
        graph: &mut Storage<'de>,
        node: VisitAlias<'de>,
    ) -> Result<NodeIndex>;

    fn visit_sequence<'de>(
        &self,
        cxt: &mut Context<'de>,
        graph: &mut Storage<'de>,
        node: VisitSequence<'de>,
    ) -> Result<NodeIndex>;

    fn visit_mapping<'de>(
        &self,
        cxt: &mut Context<'de>,
        graph: &mut Storage<'de>,
        node: VisitMapping<'de>,
    ) -> Result<NodeIndex>;
}

#[derive(Debug, Clone)]
pub(in crate::node) struct Context<'de>
{
    current: Option<NodeIndex>,

    aliases: HashMap<Slice<'de>, NodeIndex>,

    version_directive: VersionDirective,
    tag_directives:    TagDirectives<'de>,

    mapping_last_node: Option<NodeIndex>,
}

impl<'de> Context<'de>
{
    pub fn new() -> Self
    {
        Self {
            current: None,

            aliases: Default::default(),

            version_directive: DEFAULT_VERSION,
            tag_directives:    Default::default(),

            mapping_last_node: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::node) struct VisitLeaf<'de>
{
    start: usize,
    end:   usize,
    node:  ScalarEvent<'de>,
}

impl<'de> VisitLeaf<'de>
{
    pub fn from_scalar_event(start: usize, end: usize, s: ScalarEvent<'de>) -> Self
    {
        Self {
            start,
            end,
            node: s,
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::node) struct VisitAlias<'de>
{
    start: usize,
    end:   usize,
    alias: Slice<'de>,
}

impl<'de> VisitAlias<'de>
{
    pub fn from_alias_event(start: usize, end: usize, a: AliasEvent<'de>) -> Self
    {
        let alias = a.name;

        Self { start, end, alias }
    }
}

#[derive(Debug, Clone)]
pub(in crate::node) struct VisitSequence<'de>
{
    start: usize,
    end:   usize,
    node:  SequenceEvent<'de>,
}

impl<'de> VisitSequence<'de>
{
    pub fn from_sequence_event(start: usize, end: usize, s: SequenceEvent<'de>) -> Self
    {
        Self {
            start,
            end,
            node: s,
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::node) struct VisitMapping<'de>
{
    start: usize,
    end:   usize,
    node:  MappingEvent<'de>,
}

impl<'de> VisitMapping<'de>
{
    pub fn from_mapping_event(start: usize, end: usize, m: MappingEvent<'de>) -> Self
    {
        Self {
            start,
            end,
            node: m,
        }
    }
}

type ScalarEvent<'a> = crate::event::types::Node<'a, crate::event::types::ScalarLike<'a>>;
type SequenceEvent<'a> = crate::event::types::Node<'a, crate::event::types::Sequence>;
type MappingEvent<'a> = crate::event::types::Node<'a, crate::event::types::Mapping>;
type AliasEvent<'a> = crate::event::types::Alias<'a>;
