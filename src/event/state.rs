/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::mem;

pub(in crate::event) use self::flags::*;

pub(in crate::event) const INITIAL_STATE: State = State::StreamStart;
pub(in crate::event) const END_STATE: State = State::StreamEnd;

#[derive(Debug, Clone)]
pub(in crate::event) struct StateMachine
{
    top:   State,
    stack: Vec<State>,
}

impl StateMachine
{
    /// Instantiate a new state machine with the given
    /// initial State.
    pub fn new(initial: State) -> Self
    {
        Self {
            top:   initial,
            stack: Vec::default(),
        }
    }

    /// Push a State into the current .top, adding the
    /// previous .top to the stack, and returning a
    /// mutable reference to the new .top.
    pub fn push_top(&mut self, s: State) -> &mut State
    {
        let old = mem::replace(&mut self.top, s);
        self.stack.push(old);

        &mut self.top
    }

    /// Push a State onto the stack, returning a mutable
    /// reference to it.
    pub fn push(&mut self, s: State) -> &mut State
    {
        self.stack.push(s);

        self.stack.last_mut().unwrap()
    }

    /// Pop the State stack, replacing the current .top with
    /// the next State on the stack, returning the previous
    /// top if a replacement was made.
    pub fn pop(&mut self) -> Option<State>
    {
        self.stack.pop().map(|new| mem::replace(&mut self.top, new))
    }

    /// Immutably access the top State
    pub fn top(&self) -> &State
    {
        &self.top
    }

    /// Mutably access the top State
    pub fn top_mut(&mut self) -> &mut State
    {
        &mut self.top
    }

    /// Is the state machine finished?
    pub fn is_done(&self) -> bool
    {
        self.stack.is_empty() && matches!(self.top, END_STATE)
    }
}

impl Default for StateMachine
{
    fn default() -> Self
    {
        Self::new(INITIAL_STATE)
    }
}

/// Possible states in the processing of a YAML
/// [Token][crate::token::Token] sequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::event) enum State
{
    /// Expecting start of stream
    StreamStart,
    /// Expecting nothing (end of state)
    StreamEnd,

    /// Expecting start of document
    /// :: O_IMPLICIT? | O_FIRST?
    DocumentStart(Flags),
    /// Expecting document content
    DocumentContent,
    /// Expecting end of document
    DocumentEnd,

    /// Expecting a Node in the block context
    BlockNode,
    /// Expecting a Node in the flow context
    FlowNode,

    /// Expecting sequence entries in the block context
    /// :: O_FIRST?
    BlockSequenceEntry(Flags),
    /// Expecting mapping key in the block context
    /// :: O_FIRST?
    BlockMappingKey(Flags),
    /// Expecting a mapping value in the block context
    BlockMappingValue,

    /// Expecting sequence entries in the flow context
    /// :: O_FIRST?
    FlowSequenceEntry(Flags),
    /// Expecting a key in a flow sequence->mapping nested
    /// structure
    FlowSequenceMappingKey,
    /// Expecting a value in a flow sequence->mapping nested
    /// structure
    FlowSequenceMappingValue,
    /// Expecting the end of a flow sequence->mapping nested
    /// structure
    FlowSequenceMappingEnd,

    /// Expecting mapping key in the flow context
    /// :: O_FIRST?
    FlowMappingKey(Flags),
    /// Expecting a mapping value in the flow context
    /// :: O_EMPTY?
    FlowMappingValue(Flags),
}

mod flags
{
    use bitflags::bitflags;

    /// Nil / empty flag set
    pub const O_NIL: Flags = Flags::empty();
    /// Is the document implicit?
    pub const O_IMPLICIT: Flags = Flags::IMPLICIT;
    /// Is this the first entry of the sequence/mapping,
    /// or the first document in the stream?
    pub const O_FIRST: Flags = Flags::FIRST;
    /// Is the current mapping value expected to be empty?
    pub const O_EMPTY: Flags = Flags::EMPTY;

    bitflags! {
        #[derive(Default)]
        /// Options used by the state machine, not all options are relevant to all states.
        pub struct Flags: u8 {
            const IMPLICIT      = 0b00000001;
            const ALIAS         = 0b00000010;
            const TAG           = 0b00000100;
            const FIRST         = 0b00001000;
            const EMPTY         = 0b00010000;
        }
    }
}
