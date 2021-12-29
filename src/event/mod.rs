/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{
    event::{
        error::ParseResult as Result,
        state::{State, StateMachine},
        types::{Directives, Event},
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
            State::StreamStart => todo!(),
            State::DocumentStart(opts) => todo!(),
            State::DocumentContent => todo!(),
            State::DocumentEnd => todo!(),
            State::BlockNode => todo!(),
            State::FlowNode => todo!(),
            State::BlockSequenceEntry(opts) => todo!(),
            State::BlockMappingKey(opts) => todo!(),
            State::BlockMappingValue => todo!(),
            State::FlowSequenceEntry(opts) => todo!(),
            State::FlowSequenceMappingKey => todo!(),
            State::FlowSequenceMappingValue => todo!(),
            State::FlowSequenceMappingEnd => todo!(),
            State::FlowMappingKey(opts) => todo!(),
            State::FlowMappingValue(opts) => todo!(),

            // State machine terminus, no more events will be produced by this parser
            State::StreamEnd => todo!(),
        }
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
