/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{
    event::{
        error::ParseResult as Result,
        state::{Flags, State, StateMachine},
        types::{Directives, Event, NodeKind},
    },
    reader::{PeekReader, Read},
};

#[macro_use]
mod macros;

mod state;

pub mod error;
pub mod types;

type Tokens<'de, T> = PeekReader<'de, T>;

/// The [Parser] provides an API for translating any
/// [Token] [Read] stream into higher level [Event]s.
///
/// The two primary methods of of interest on this
/// struct are:
///
/// 1. [next_event](#method.next_event)
/// 2. [into_iter](#method.into_iter)
///
/// The first provides an interface for retrieving the
/// next [Event] from the given [Read]er, while the
/// latter provides an [Iterator] based interface to
/// retrieve [Event]s from, reusing the provided [Read]er.
///
/// A Parser can iteratively consume an entire [Token]
/// stream ending when `Token::StreamEnd` is found, after
/// which the Parser considers the stream finished and
/// always returns None.
///
/// [Token]: enum@crate::token::Token
/// [Read]: trait@crate::reader::Read
#[derive(Debug, Clone)]
pub struct Parser
{
    state: StateMachine,

    directives: Directives<'static>,
    done:       bool,
}

impl Parser
{
    /// Instantiate a new [Parser], ready for a new token
    /// stream.
    pub fn new() -> Self
    {
        Self {
            state:      StateMachine::default(),
            directives: Default::default(),
            done:       false,
        }
    }

    /// Fetch the next [Event] from the provided .tokens
    /// stream.
    ///
    /// Note that once you call this method, the associated
    /// .tokens is "bound" to this [Parser], and should not
    /// be provided to anything else which modifies the
    /// stream, including a different [Parser].
    pub fn next_event<'de, T>(&mut self, tokens: &mut Tokens<'de, T>) -> Option<Result<Event<'de>>>
    where
        T: Read,
    {
        self.get_next_event(tokens).transpose()
    }

    /// Provides an [Iterator] interface to this [Parser],
    /// via the given .tokens
    #[allow(clippy::wrong_self_convention)]
    pub fn into_iter<'a, 'b, 'de, T>(
        &'a mut self,
        tokens: &'b mut Tokens<'de, T>,
    ) -> EventIter<'a, 'b, 'de, T>
    where
        T: Read,
    {
        EventIter::new(self, tokens)
    }

    /// Runs the state machine until it either provides the
    /// next [Event], an error, or the state machine is
    /// finished
    fn get_next_event<'de, T>(&mut self, tokens: &mut Tokens<'de, T>) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        let mut event = None;

        // Main loop, continue until an event is produced, an error
        // is returned or we're marked as finished.
        while !self.done && event.is_none()
        {
            event = self.state_transition(tokens)?;
        }

        Ok(event)
    }

    /// Process the next event in the state machine, running
    /// the associated routine
    fn state_transition<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        match *self.state.top()
        {
            State::StreamStart => self.stream_start(tokens),
            State::DocumentStart(opts) => self.document_start(tokens, opts),
            State::DocumentContent => self.explicit_document_content(tokens),
            State::DocumentEnd => self.document_end(tokens),
            State::BlockNode => self.node(tokens, BLOCK_CONTEXT, NodeKind::Root),
            State::FlowNode => self.node(tokens, !BLOCK_CONTEXT, NodeKind::Root),
            State::BlockSequenceEntry(opts) => self.block_sequence_entry(tokens, opts),
            State::BlockMappingKey(opts) => self.block_mapping_key(tokens, opts),
            State::BlockMappingValue => self.block_mapping_value(tokens),
            State::FlowSequenceEntry(opts) => self.flow_sequence_entry(tokens, opts),
            State::FlowSequenceMappingKey => self.flow_sequence_entry_mapping_key(tokens),
            State::FlowSequenceMappingValue => self.flow_sequence_entry_mapping_value(tokens),
            State::FlowSequenceMappingEnd => self.flow_sequence_entry_mapping_end(tokens),
            State::FlowMappingKey(opts) => self.flow_mapping_key(tokens, opts),
            State::FlowMappingValue(opts) => self.flow_mapping_value(tokens, opts),

            // State machine terminus, no more events will be produced by this parser
            State::StreamEnd => self.stream_end(tokens),
        }
    }

    /// Start of token stream, ensure the underlying Read
    /// stream hasn't been tampered with, and return the
    /// associated Event
    fn stream_start<'de, T>(&mut self, tokens: &mut Tokens<'de, T>) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// End of token stream, set ourself to done and produce
    /// the associated Event, if we haven't already
    fn stream_end<'de, T>(&mut self, tokens: &mut Tokens<'de, T>) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Start of a new document, process any directives,
    /// determine if it's explicit and prime the state
    /// machine accordingly, returning the associated
    /// Event if appropriate
    fn document_start<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
        opts: Flags,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// End of document, determine if its explicit, and
    /// return the associated Event
    fn document_end<'de, T>(&mut self, tokens: &mut Tokens<'de, T>) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Handle an explicit, maybe empty document returning
    /// the root node [Event] if appropriate, or nothing
    /// if the document is empty.
    fn explicit_document_content<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Block context sequence entry, return the associated
    /// node or sequence end [Event]
    fn block_sequence_entry<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
        opts: Flags,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Block context mapping key, return the appropriate
    /// node or mapping end [Event], pushing a mapping value
    /// state to the stack in the former case
    fn block_mapping_key<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
        opts: Flags,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Block context mapping value, return the appropriate
    /// node or mapping end [Event], pushing a mapping key
    /// state to the stack in the former case
    fn block_mapping_value<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Flow context sequence entry, return the associated
    /// node or sequence end [Event]
    fn flow_sequence_entry<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
        opts: Flags,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Flow mapping key with parent flow sequence, return
    /// the associated node [Event] and prep the tight state
    /// loop for flow_sequence->flow_mapping token sequences
    fn flow_sequence_entry_mapping_key<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Flow mapping value with parent flow sequence, return
    /// the associated node [Event] and push a
    /// FlowSequenceMappingEnd to the state stack.
    ///
    /// Note it is an invariant of this function that it
    /// must *always* push the above state to the stack
    /// -- excluding in error cases.
    fn flow_sequence_entry_mapping_value<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Clean up after a flow_sequence->flow_mapping state
    /// loop, returning the appropriate mapping end [Event]
    fn flow_sequence_entry_mapping_end<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Flow context mapping key, return the appropriate
    /// node or mapping end [Event], pushing a mapping value
    /// state to the stack in the former case
    fn flow_mapping_key<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
        opts: Flags,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Flow context mapping value, return the appropriate
    /// node or mapping end [Event]
    fn flow_mapping_value<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
        opts: Flags,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Produce a node or alias [Event]
    fn node<'de, T>(
        &mut self,
        tokens: &mut Tokens<'de, T>,
        block: bool,
        kind: NodeKind,
    ) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        todo!()
    }

    /// Produce an empty scalar node [Event], always returns
    /// Ok, the Result is mostly for compose-ability
    fn empty_scalar(&mut self, mark: usize, kind: NodeKind) -> Result<Event<'static>>
    {
        todo!()
    }
}

/// Provides an [Iterator] interface to interact with
/// [Event]s through.
#[derive(Debug)]
pub struct EventIter<'a, 'b, 'de, T>
{
    parser: &'a mut Parser,
    reader: &'b mut Tokens<'de, T>,
}

impl<'a, 'b, 'de, T> EventIter<'a, 'b, 'de, T>
where
    T: Read,
{
    fn new(parser: &'a mut Parser, reader: &'b mut Tokens<'de, T>) -> Self
    {
        Self { parser, reader }
    }
}

impl<'a, 'b, 'de, T> Iterator for EventIter<'a, 'b, 'de, T>
where
    T: Read,
{
    type Item = Result<Event<'de>>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.parser.next_event(self.reader)
    }
}

const BLOCK_CONTEXT: bool = true;
