/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::array::IntoIter as ArrayIter;

use crate::{
    event::{
        error::{ParseError as Error, ParseResult as Result},
        state::{Flags, State, StateMachine, O_FIRST, O_IMPLICIT, O_NIL},
        types::{Directives, Event, EventData, NodeKind, DEFAULT_TAGS, EMPTY_SCALAR},
    },
    reader::{PeekReader, Read},
    token::{Marker, Slice},
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
        let token = peek!(~tokens)?;

        let event = match token
        {
            Marker::StreamStart => initEvent!(@consume StreamStart => tokens),
            _ => Err(Error::CorruptStream),
        }?;

        state!(~self, -> State::DocumentStart(O_IMPLICIT | O_FIRST));

        Ok(Some(event))
    }

    /// End of token stream, set ourself to done and produce
    /// the associated Event, if we haven't already
    fn stream_end<'de, T>(&mut self, tokens: &mut Tokens<'de, T>) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        if self.done
        {
            return Ok(None);
        }

        let event = initEvent!(@consume StreamEnd => tokens).map(Some)?;
        self.done = true;

        Ok(event)
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
        let mut event = None;
        let implicit = opts.contains(O_IMPLICIT);
        let first = opts.contains(O_FIRST);

        // If the document is explicit we need to skip any extra
        // DocumentEnd tokens ('...')
        if !implicit
        {
            while peek!(~tokens)? == Marker::DocumentEnd
            {
                pop!(tokens)?;
            }
        }

        let token = peek!(~tokens)?;
        let markers = matches!(
            token,
            Marker::TagDirective
                | Marker::VersionDirective
                | Marker::DocumentStart
                | Marker::StreamEnd
        );

        // Implicit, non empty document, no directives
        if implicit && !markers
        {
            // Retrieve any directives for the current document, merged
            // with the defaults
            let (start, end, directives) =
                scan_document_directives(tokens, ArrayIter::new(DEFAULT_TAGS))?;

            let Directives { version, tags } = directives;
            event =
                initEvent!(@event DocumentStart => (start, end, (version, tags, !EXPLICIT))).into();

            // Enqueue State.DocumentEnd, set active to State.BlockNode
            state!(~self, >> State::DocumentEnd, -> State::BlockNode);
        }
        // Explicit document, maybe with directives
        else if !matches!(token, Marker::StreamEnd)
        {
            // Retrieve any directives for the current document, merged
            // with the defaults
            let (start, _, directives) =
                scan_document_directives(tokens, ArrayIter::new(DEFAULT_TAGS))?;

            // Ensure we have an explicit DocumentStart indicator
            let end = match peek!(~tokens)?
            {
                Marker::DocumentStart => pop!(tokens).map(|entry| entry.read_at()),
                _ => Err(Error::MissingDocumentStart),
            }?;

            let Directives { version, tags } = directives;
            event =
                initEvent!(@event DocumentStart => (start, end, (version, tags, EXPLICIT))).into();

            // Enqueue State.DocumentEnd, set active to
            // State.DocumentContent
            state!(~self, >> State::DocumentEnd, -> State::DocumentContent);
        }
        // We always return at least one document event pair, even if the stream is empty
        else if first
        {
            let (start, end, directives) =
                scan_document_directives(tokens, ArrayIter::new(DEFAULT_TAGS))?;

            let Directives { version, tags } = directives;
            event =
                initEvent!(@event DocumentStart => (start, end, (version, tags, !EXPLICIT))).into();

            // document_end returns control to us after event
            // production, so our stream_end branch (below)
            // will fire and activate stream_end
            state!(~self, -> State::DocumentEnd);
        }
        // Stream end, transition state machine to final state
        else
        {
            state!(~self, -> State::StreamEnd);
        }

        // Set the Parser's active directives to the
        // upcoming document's
        if let Some(EventData::DocumentStart(doc)) = event.as_ref().map(|event| event.data())
        {
            let version = doc.directives.version;
            let tags = doc.directives.tags.iter().map(tags_to_owned).collect();

            self.directives = Directives { version, tags };
        }

        Ok(event)
    }

    /// End of document, determine if its explicit, and
    /// return the associated Event
    fn document_end<'de, T>(&mut self, tokens: &mut Tokens<'de, T>) -> Result<Option<Event<'de>>>
    where
        T: Read,
    {
        let (event, opts);
        let (start, mut end, token) = peek!(tokens)?;
        let mut implicit = true;

        if matches!(token, Marker::DocumentEnd)
        {
            implicit = false;
            pop!(tokens)?;
        }
        else
        {
            // If the token isn't a DocumentEnd, then this Event is
            // "virtual" and has no real length
            end = start;
        }

        // If this the DocumentEnd was implicit then the next
        // document start must be explicit
        opts = implicit.then(|| O_NIL).unwrap_or(O_IMPLICIT);
        state!(~self, -> State::DocumentStart(opts));

        event = initEvent!(@event DocumentEnd => (start, end, implicit));

        Ok(Some(event))
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
        use Marker::*;
        let token = peek!(~tokens)?;

        // Check if the next token indicates an empty document
        let empty = matches!(
            token,
            VersionDirective | TagDirective | DocumentStart | DocumentEnd | StreamEnd
        );

        // The document might be empty, in which case skip event
        // production, pop the state stack and return control to the
        // state machine loop
        if empty
        {
            // Pop the state stack
            state!(~self, << None);

            Ok(None)
        }
        // Otherwise, process the document's node graph
        else
        {
            self.node(tokens, BLOCK_CONTEXT, NodeKind::Root)
        }
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
        let event =
            initEvent!(@event Scalar => (mark, mark, (NO_ANCHOR, NO_TAG, kind, EMPTY_SCALAR)));

        Ok(event)
    }
}

/// Fetch all adjacent YAML directives from the stream,
/// merging them with the provided default_directives,
/// returning the the start + end stream marks, and the
/// directives themselves.
fn scan_document_directives<'a: 'de, 'de, I, T>(
    tokens: &mut Tokens<'de, T>,
    default_directives: I,
) -> Result<(usize, usize, Directives<'de>)>
where
    I: Iterator<Item = (Slice<'a>, Slice<'a>)>,
    T: Read,
{
    #[allow(unused_assignments)]
    let (start, mut end, mut token) = peek!(tokens)?;

    let mut directives = Directives::default();
    let mut seen_version = false;

    let tags = &mut directives.tags;
    let version = &mut directives.version;

    loop
    {
        token = peek!(~tokens)?;

        match token
        {
            Marker::VersionDirective if !seen_version =>
            {
                seen_version = true;

                *version = {
                    let (_, new_end, version) = consume!(tokens, VersionDirective)?;
                    end = new_end;

                    version
                }
            },
            Marker::VersionDirective => return Err(Error::DuplicateVersion),

            Marker::TagDirective =>
            {
                let (_, new_end, (handle, prefix)) = consume!(tokens, TagDirective)?;

                /*
                 * %TAG directives with the same handle are an error
                 *
                 * See:
                 *  yaml.org/spec/1.2.2/#682-tag-directives
                 */
                if tags.get(&handle).is_some()
                {
                    return Err(Error::DuplicateTagDirective);
                }

                end = new_end;
                tags.insert(handle, prefix);
            },

            _ => break,
        }
    }

    // Insert any missing default directives, but do not
    // overwrite existing values
    default_directives.for_each(|(handle, prefix)| {
        tags.entry(handle).or_insert(prefix);
    });

    Ok((start, end, directives))
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

const EXPLICIT: bool = false;
const BLOCK_CONTEXT: bool = true;
const NO_ANCHOR: Option<Slice<'static>> = None;
const NO_TAG: Option<(Slice<'static>, Slice<'static>)> = None;
