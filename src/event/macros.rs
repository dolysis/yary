/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

/// Peek the head of the .queue, returning its start and end
/// byte and a marker representing the underlying Token, in
/// a three item tuple (.start, .end, .marker)
///
/// Modifiers
///     ~  .queue := return .marker
///     @~ .queue := return Option<.marker> (no error)
///
/// Variants
///     /1 .queue
macro_rules! peek {
    ($queue:expr) => {
        $queue
            .peek()
            .map_err(Into::into)
            .and_then(|maybe| maybe.ok_or($crate::event::error::ParseError::UnexpectedEOF))
            .map(|entry| (entry.read_at(), entry.read_at(), entry.marker()))
    };
    (~ $queue:expr) => {
        $queue
            .peek()
            .map_err(Into::into)
            .and_then(|maybe| maybe.ok_or($crate::event::error::ParseError::UnexpectedEOF))
            .map(|entry| entry.marker())
    };
    (@ ~ $queue:expr) => {
        $queue
            .peek()
            .map_err($crate::event::error::ParseError::from)
            .map(|maybe| maybe.map(|entry| entry.marker()))
    };
}

/// Pop the head of the .queue, returning the entry, or an
/// error if the queue was empty. Typically
/// used in combination with peek!
///
/// Variants
///     /1 .queue
macro_rules! pop {
    ($queue:expr) => {
        $queue
            .pop()
            .map_err(Into::into)
            .and_then(|maybe| maybe.ok_or($crate::event::error::ParseError::UnexpectedEOF))
    };
}

/// ```text
/// Manipulate the given state .machine (or .parser),
/// pushing / popping states in the stack and modifying the
/// current top state
///
/// Variants
///     /1 .machine, $op .state
///     /2 .parser, $op .state *[, $op .state ]
///
///     $op :=
///         | -> (change top state)
///         | >> (push state to stack)
///         | |> (change top state, add old top to stack)
///         | << (pop state from stack to top)
/// ```
macro_rules! state {
    (~$parser:expr, $( $op:tt $state:expr ),+) => {
        $( state!($parser.state, $op $state); )+
    };

    ($machine:expr, -> $state:expr) => {
        *$machine.top_mut() = $state
    };
    ($machine:expr, >> $state:expr) => {
        $machine.push($state)
    };
    ($machine:expr, |> $state:expr) => {
        $machine.push_top($state)
    };
    ($machine:expr, << $_:expr) => {
        $machine.pop()
    };
}

/// ```text
/// Consume an entry of $kind from the .queue, returning its
/// (start, end, context), or an error. Note that the exact
/// nature of context varies.
///
/// Variants
///     /1 .queue, $kind
///
///     $kind :=
///         | StreamStart
///         | StreamEnd
///         | VersionDirective
///         | TagDirective
///         | Alias
///         | Anchor
///         | Tag
///         | Scalar
///         | FlowSequenceStart
///         | FlowMappingStart
///         | BlockSequenceStart
///         | BlockMappingStart
/// ```
macro_rules! consume {
    ($queue:expr, $kind:tt) => {{
        #[allow(unused_imports)]
        use $crate::{token::Token::*, scanner::entry::MaybeToken, event::types::{Event, EventData, VersionDirective, Scalar}};

        #[allow(clippy::collapsible_match)]
        pop!($queue).map(|entry| consume!(@wrap entry, $kind))
    }};

    (@wrap $entry:expr, Scalar) => {{
        let end = $entry.read_at();

        match $entry.wrap {
            MaybeToken::Token(token) => match token {
                Scalar(data, style) => (end, end, Scalar::Eager { data, style }),
                _ => unreachable!(),
            },
            MaybeToken::Deferred(lazy) => (end, end, Scalar::Lazy { data: Box::new(lazy) })
        }
    }};
    (@wrap $entry:expr, $kind:tt) => {{
        let end = $entry.read_at();

        match $entry.wrap {
            MaybeToken::Token(token) => consume!(@entry $kind => end, end, token),
            _ => unreachable!(),
        }
    }};

    (@entry StreamStart => $start:expr, $end:expr, $token:expr) => {
        match $token {
            StreamStart(encoding) => ($start, $end, encoding),
            _ => unreachable!(),
        }
    };
    (@entry StreamEnd => $start:expr, $end:expr, $token:expr) => {
        match $token {
            StreamEnd => ($start, $end, ()),
            _ => unreachable!(),
        }
    };
    (@entry VersionDirective => $start:expr, $end:expr, $token:expr) => {
        match $token {
            VersionDirective(major, minor) => ($start, $end, VersionDirective { major, minor }),
            _ => unreachable!(),
        }
    };
    (@entry TagDirective => $start:expr, $end:expr, $token:expr) => {
        match $token {
            TagDirective(handle, prefix) => ($start, $end, (handle, prefix)),
            _ => unreachable!(),
        }
    };
    (@entry Alias => $start:expr, $end:expr, $token:expr) => {
        match $token {
            Alias(name) => ($start, $end, name),
            _ => unreachable!(),
        }
    };
    (@entry Anchor => $start:expr, $end:expr, $token:expr) => {
        match $token {
            Anchor(name) => ($start, $end, name),
            _ => unreachable!(),
        }
    };
    (@entry Tag => $start:expr, $end:expr, $token:expr) => {
        match $token {
            Tag(handle, suffix) => ($start, $end, (handle, suffix)),
            _ => unreachable!(),
        }
    };
    (@entry FlowSequenceStart => $start:expr, $end:expr, $token:expr) => {
        match $token {
            FlowSequenceStart => ($start, $end, ()),
            _ => unreachable!(),
        }
    };
    (@entry FlowMappingStart => $start:expr, $end:expr, $token:expr) => {
        match $token {
            FlowMappingStart => ($start, $end, ()),
            _ => unreachable!(),
        }
    };
    (@entry BlockSequenceStart => $start:expr, $end:expr, $token:expr) => {
        match $token {
            BlockSequenceStart => ($start, $end, ()),
            _ => unreachable!(),
        }
    };
    (@entry BlockMappingStart => $start:expr, $end:expr, $token:expr) => {
        match $token {
            BlockMappingStart => ($start, $end, ()),
            _ => unreachable!(),
        }
    };
}

/// ```text
/// Generate a new event of $kind from the given .context,
/// or consume it from the provided .queue.
///
/// Variants
///     /1 @event $kind => .context
///     /2 @consume $kind => .queue
///
///     $kind :=
///         | StreamStart
///         | StreamEnd
///         | DocumentStart
///         | DocumentEnd
///         | Alias
///         | Scalar
///         | BlockSequenceStart
///         | BlockMappingStart
///         | FlowSequenceStart
///         | FlowMappingStart
///         | SequenceEnd
///         | MappingEnd
/// ```
macro_rules! initEvent {
    (@consume $kind:tt => $queue:expr) => {{
        #[allow(unused_imports)]
        use $crate::{event::types::{self, EventData}};

        consume!($queue, $kind).map(|context| initEvent!(@event $kind => context))
    }};

    (@event StreamStart => $context:expr) => {{
        let (start, end, encoding) = $context;

        Event::new(start, end, EventData::StreamStart(types::StreamStart { encoding }))
    }};
    (@event StreamEnd => $context:expr) => {{
        let (start, end, ()) = $context;

        Event::new(start, end, EventData::StreamEnd)
    }};
    (@event DocumentStart => $context:expr) => {{
        let (start, end, (version, tags, implicit)) = $context;
        let directives = types::Directives { version, tags };

        Event::new(start, end, EventData::DocumentStart(types::DocumentStart { directives, implicit }))
    }};
    (@event DocumentEnd => $context:expr) => {{
        let (start, end, implicit) = $context;

        Event::new(start, end, EventData::DocumentEnd(types::DocumentEnd { implicit }))
    }};
    (@event SequenceEnd => $context:expr) => {{
        let (start, end, _) = $context;

        Event::new(start, end, EventData::SequenceEnd)
    }};
    (@event MappingEnd => $context:expr) => {{
        let (start, end, _) = $context;

        Event::new(start, end, EventData::MappingEnd)
    }};
    (@event Alias => $context:expr) => {{
        let (start, end, name) = $context;

        Event::new(start, end, EventData::Alias(types::Alias { name }))
    }};
    (@event FlowSequenceStart => $context:expr) => {{
        let (start, end, (anchor, tag, kind)) = $context;

        Event::new(start, end, EventData::SequenceStart(types::Node { anchor, tag, content: types::Sequence, kind }))
    }};
    (@event FlowMappingStart => $context:expr) => {{
        let (start, end, (anchor, tag, kind)) = $context;

        Event::new(start, end, EventData::MappingStart(types::Node { anchor, tag, content: types::Mapping, kind }))
    }};
    (@event BlockSequenceStart => $context:expr) => {{
        let (start, end, (anchor, tag, kind)) = $context;

        Event::new(start, end, EventData::SequenceStart(types::Node { anchor, tag, content: types::Sequence, kind }))
    }};
    (@event BlockMappingStart => $context:expr) => {{
        let (start, end, (anchor, tag, kind)) = $context;

        Event::new(start, end, EventData::MappingStart(types::Node { anchor, tag, content: types::Mapping, kind }))
    }};
    (@event Scalar => $context:expr) => {{
        let (start, end, (anchor, tag, kind, content)) = $context;

        Event::new(start, end, EventData::Scalar(types::Node { anchor, tag, content, kind }))
    }};
}
