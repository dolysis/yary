/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{
    event::{
        self,
        types::{DocumentStart, Event, EventData},
        Events,
    },
    node::{
        error::NodeResult as Result,
        graph::Storage,
        nodes::NodeIndex,
        visitor::{
            AliasEvent, Context, MappingEvent, ScalarEvent, SequenceEvent, VisitAlias, VisitLeaf,
            VisitMapping, VisitSequence, Visitor,
        },
    },
    reader::Read,
};

pub(in crate::node) struct StreamConsumer<'de, R, V>
{
    events: EventIter<'de, R>,

    graph: Storage<'de>,

    visitor:       V,
    context:       Context<'de>,
    context_stack: Vec<SavedContext>,
}

impl<'de, R, V> StreamConsumer<'de, R, V>
where
    V: Visitor,
    R: Read,
{
    pub fn new(events: Events<'de, R>, visitor: V) -> Self
    {
        Self {
            events: EventIter::new(events),
            graph: Storage::new(),
            visitor,
            context: Context::new(),
            context_stack: Default::default(),
        }
    }

    pub fn from_src(src: &'de R, visitor: V) -> Self
    {
        let events = event::from_reader(src);

        Self::new(events, visitor)
    }

    /// Consumes the internal event stream until the next
    /// node has been stored, or an error is encountered.
    ///
    /// Returns the id from the node, or None if the event
    /// stream is finished.
    fn next_node(&mut self) -> Result<Option<NodeIndex>>
    {
        let mut node_id = None;

        while let Some(mut event) = self.events.next().transpose()?
        {
            let (start, end) = (event.start(), event.end());

            let node = match take_event_data(event.data_mut())
            {
                EventData::StreamStart(_) => self.init_context(),
                EventData::DocumentStart(doc) => self.set_doc_context(doc),
                EventData::DocumentEnd(_) => self.clear_doc_context(),
                EventData::Alias(alias) => self.insert_alias(start, end, alias).map(Some)?,
                EventData::Scalar(scalar) => self.insert_scalar(start, end, scalar).map(Some)?,
                EventData::MappingStart(mapping) =>
                {
                    self.insert_mapping(start, end, mapping).map(Some)?
                },
                EventData::SequenceStart(sequence) =>
                {
                    self.insert_sequence(start, end, sequence).map(Some)?
                },
                EventData::MappingEnd | EventData::SequenceEnd => self.decrement_context_level(),
                EventData::StreamEnd => break,
            };

            if let Some(id) = node
            {
                node_id = Some(id);
                break;
            }
        }

        Ok(node_id)
    }

    fn init_context(&mut self) -> Option<NodeIndex>
    {
        None
    }

    fn set_doc_context(&mut self, doc: DocumentStart<'de>) -> Option<NodeIndex>
    {
        self.context.version_directive = doc.directives.version;
        self.context.tag_directives = doc.directives.tags;

        None
    }

    fn clear_doc_context(&mut self) -> Option<NodeIndex>
    {
        let aliases = &mut self.context.aliases;
        let tags = &mut self.context.tag_directives;

        aliases.clear();
        aliases.shrink_to_fit();

        tags.clear();
        tags.shrink_to_fit();

        None
    }

    fn insert_alias(&mut self, start: usize, end: usize, node: AliasEvent<'de>)
        -> Result<NodeIndex>
    {
        let v_alias = VisitAlias::from_alias_event(start, end, node);

        let id = self
            .visitor
            .visit_alias(&mut self.context, &mut self.graph, v_alias)?;

        Ok(id)
    }

    fn insert_scalar(
        &mut self,
        start: usize,
        end: usize,
        node: ScalarEvent<'de>,
    ) -> Result<NodeIndex>
    {
        let v_leaf = VisitLeaf::from_scalar_event(start, end, node);

        let id = self
            .visitor
            .visit_leaf(&mut self.context, &mut self.graph, v_leaf)?;

        Ok(id)
    }

    fn insert_sequence(
        &mut self,
        start: usize,
        end: usize,
        node: SequenceEvent<'de>,
    ) -> Result<NodeIndex>
    {
        let v_sequence = VisitSequence::from_sequence_event(start, end, node);

        self.increment_context_level();

        let id = self
            .visitor
            .visit_sequence(&mut self.context, &mut self.graph, v_sequence)?;

        Ok(id)
    }

    fn insert_mapping(
        &mut self,
        start: usize,
        end: usize,
        node: MappingEvent<'de>,
    ) -> Result<NodeIndex>
    {
        let v_mapping = VisitMapping::from_mapping_event(start, end, node);

        self.increment_context_level();

        let id = self
            .visitor
            .visit_mapping(&mut self.context, &mut self.graph, v_mapping)?;

        Ok(id)
    }

    fn increment_context_level(&mut self) -> Option<NodeIndex>
    {
        let save = SavedContext::save(&mut self.context);

        self.context_stack.push(save);

        None
    }

    fn decrement_context_level(&mut self) -> Option<NodeIndex>
    {
        if let Some(saved) = self.context_stack.pop()
        {
            saved.unsave(&mut self.context)
        }

        None
    }
}

struct SavedContext
{
    current: Option<NodeIndex>,

    mapping_last_node: Option<NodeIndex>,
}

impl SavedContext
{
    fn save(cxt: &mut Context) -> Self
    {
        let current = cxt.current.take();
        let mln = cxt.mapping_last_node.take();

        Self::new(current, mln)
    }

    fn unsave(self, cxt: &mut Context)
    {
        cxt.current = self.current;
        cxt.mapping_last_node = self.mapping_last_node;
    }

    fn new(current_node: Option<NodeIndex>, mapping_last_node: Option<NodeIndex>) -> Self
    {
        Self {
            current: current_node,
            mapping_last_node,
        }
    }
}

#[derive(Debug)]
struct EventIter<'de, R>
{
    inner: Events<'de, R>,

    done: bool,
}

impl<'de, R> EventIter<'de, R>
where
    R: Read,
{
    fn new(events: Events<'de, R>) -> Self
    {
        Self {
            inner: events,
            done:  false,
        }
    }

    fn into_inner(self) -> Events<'de, R>
    {
        self.inner
    }
}

impl<'de, R> Iterator for EventIter<'de, R>
where
    R: Read,
{
    type Item = Result<Event<'de>>;

    fn next(&mut self) -> Option<Self::Item>
    {
        if self.done
        {
            return None;
        }

        let event = self
            .inner
            .iter()
            .next_event()
            .map_err(Into::into)
            .transpose();

        event.is_none().then(|| self.done = true);

        event
    }
}

impl<'de, R> std::iter::FusedIterator for EventIter<'de, R> where R: Read {}

fn take_event_data<'de>(event: &mut EventData<'de>) -> EventData<'de>
{
    std::mem::replace(event, EventData::StreamEnd)
}
