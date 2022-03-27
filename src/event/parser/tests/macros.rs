/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

/// Generate a PeekQueue instance from the given .tokens.
/// Note that the returned TokenEntry's read_at will be 0.
///
/// Usage:
///     /1 +[ .token, ...]
macro_rules! tokens {
    ($($token:expr),+) => {{
        use std::iter::FromIterator;
        use $crate::{queue::Queue, scanner::entry::TokenEntry};

        let tokens = vec![ $( $token ),+ ]
            .into_iter()
            .map(|token| TokenEntry::new(token, 0));

        Queue::from_iter(tokens)
    }};
}

#[rustfmt::skip]
/// Generate an Event from the given $type, optionally setting
/// the .start and .end marks.
///
/// Variants
///     /1 { $type }, .start, .end
///     /2 { $type } := /1 { $type }, 0, 0
///
///     $type :=
///         | StreamStart ?[.encoding]
///         | StreamEnd
///         | DocumentStart ?[@explicit] ?[.major, .minor] ?[ [ *[.handle, .prefix] ] ]
///         | DocumentEnd ?[@explicit]
///         | Anchor .name
///         | Scalar .scalar
///         | MappingStart @.kind
///         | MappingEnd
///         | SequenceStart @.kind
///         | SequenceEnd
///
macro_rules! event {
    ($args:tt $(, $start:expr)? $(=> $end:expr)? ) => {{
        #[allow(unused_imports)]
        use types::{Event, EventData, self};

        let (start, end) = event!(@marks $($start ,)? 0 => $($end ,)? 0);
        Event::new(start, end, event!(@type $args))
    }};

    (@type {StreamStart $( $encoding:expr )? }) => {
        EventData::StreamStart(types::StreamStart {
            encoding: event!(@option $( $encoding ,)? $crate::token::StreamEncoding::UTF8)
        })
    };
    (@type {StreamEnd}) => {
        EventData::StreamEnd
    };
    (@type {DocumentStart $(@ $explicit:tt )? $( $major:literal , $minor:literal )? $( [ $({$handle:expr, $prefix:expr}),* ] )? }) => {
        EventData::DocumentStart(
            types::DocumentStart {
                directives: types::Directives {
                    version: event!(@option
                        $( types::VersionDirective { major: $major, minor: $minor } ,)?
                        types::DEFAULT_VERSION
                    ),
                    tags: std::iter::FromIterator::from_iter(
                        std::array::IntoIter::new(types::DEFAULT_TAGS).chain(vec![
                                $($( ($crate::token::Slice::from($handle), $crate::token::Slice::from($prefix)) ),*)?
                    ])),
                },
                implicit: !event!(@explicit $( $explicit ,)? implicit),
            }
        )
    };
    (@type {DocumentEnd $(@ $explicit:tt)? }) => {
        EventData::DocumentEnd(
            types::DocumentEnd { implicit: !event!(@explicit $( $explicit ,)? implicit) }
        )
    };
    (@type {Anchor $name:expr }) => {
        EventData::Anchor(types::Anchor { name: $name })
    };
    (@type {Scalar $scalar:expr }) => {
        EventData::Scalar($scalar)
    };
    (@type {MappingStart @$kind:tt $(& $anchor:expr ,)? $(@ $handle:expr, $suffix:expr)? }) => {
        EventData::MappingStart(types::Node {
            anchor: event!(@option $( Some($anchor.into()) ,)? None),
            tag: event!(@option $( Some(($handle.into(), $suffix.into())) ,)? None),
            content: types::Mapping,
            kind: event!(@kind $kind),
        })
    };
    (@type {MappingEnd}) => {
        EventData::MappingEnd
    };
    (@type {SequenceStart @$kind:tt $(& $anchor:expr ,)? $(@ $handle:expr, $suffix:expr)? }) => {
        EventData::SequenceStart(types::Node {
            anchor: event!(@option $( Some($anchor.into()) ,)? None),
            tag: event!(@option $( Some(($handle.into(), $suffix.into())) ,)? None),
            content: types::Sequence,
            kind: event!(@kind $kind),
        })
    };
    (@type {SequenceEnd}) => {
        EventData::SequenceEnd
    };

    (@marks $start:expr $(, $_:expr )? => $end:expr $(, $__:expr)?) => { ($start, $end) };

    (@option $return:expr $(, $_:expr)? ) => { $return };

    (@explicit explicit $(, $_op:tt )? ) =>  { true };
    (@explicit $_:tt $(, $_op:tt )? ) => { false };

    (@kind Root) => { types::NodeKind::Root };
    (@kind Entry) => { types::NodeKind::Entry };
    (@kind Key) => { types::NodeKind::Key };
    (@kind Value) => { types::NodeKind::Value };
}

/// Generate a Node from the given .content and .kind, with
/// an optional .tag and/or .alias.
///
/// Variants
///     /1 .content, @ .kind & .alias, @ .tag
///     /2 .content, @ .kind & .alias
///         := /1 .content, @ .kind & .alias, @ None
///     /3 .content, @ .kind @ .tag
///         := /1 .content, @ .kind & None, @ .tag
///     /4 .content @ .kind
///         := /1 .content, @ .kind & None, @ None
macro_rules! node {
    ($content:expr, @$kind:tt) => {
        types::Node {
            anchor:  None,
            tag:     None,
            content: $content,
            kind: node!(@kind $kind),
        }
    };
    ($content:expr, @$kind:tt @ $handle:expr, $suffix:expr) => {
        types::Node {
            anchor:  None,
            tag:     Some((
                $crate::token::Slice::from($handle),
                $crate::token::Slice::from($suffix),
            )),
            content: $content,
            kind: node!(@kind $kind),
        }
    };
    ($content:expr, @$kind:tt & $alias:expr) => {
        types::Node {
            anchor:  Some($crate::token::Slice::from($alias)),
            tag:     None,
            content: $content,
            kind: node!(@kind $kind),
        }
    };
    ($content:expr, @$kind:tt & $alias:expr, @ $handle:expr, $suffix:expr) => {
        types::Node {
            anchor:  Some($crate::token::Slice::from($alias)),
            tag:     Some((
                $crate::token::Slice::from($handle),
                $crate::token::Slice::from($suffix),
            )),
            content: $content,
            kind: node!(@kind $kind),
        }
    };

    (@kind Root) => { types::NodeKind::Root };
    (@kind Entry) => { types::NodeKind::Entry };
    (@kind Key) => { types::NodeKind::Key };
    (@kind Value) => { types::NodeKind::Value };
}

/// Generate a Scalar from the given string .content and
/// scalar .style
///
/// Modifiers
///     ~ := Content::Scalar( ... )
///
/// Variants
///     /1 .content, .style
///     /2 .content := /1 .content, ScalarStyle::Plain
macro_rules! scalar {
    (~ $content:expr $(, $style:expr)? ) => {
        types::Content::Scalar( scalar!($content $(, $style)?) )
    };
    ($content:expr) => {
        types::Scalar::Eager {
            data:  $crate::token::Slice::from($content),
            style: $crate::token::ScalarStyle::Plain,
        }
    };
    ($content:expr, $style:expr) => {
        types::ScalarLike::eager(
            $crate::token::Slice::from($content), $style
        )
    };
}

/// Generate a Slice from the given .content
///
/// Variants
///     /1 .content
macro_rules! cow {
    ($content:expr) => {
        $crate::token::Slice::from($content)
    };
}

/// Test harness for Events. Takes the given PeekQueue
/// .tokens and tests a Parser's output Events against the
/// given .match set, optionally taking a context .msg.
///
/// Variants
///     /1 .tokens => +[ $match ?[=> .msg], ]
///
///     $match :=
///         | | .event
///         | @ .option(Event)
///         | > .result(Event)
macro_rules! events {
    ($tokens:expr => $($op:tt $match:expr $(=> $msg:expr)?),+) => {{
        use $crate::{reader::{borrow::BorrowReader, Reader, PeekReader}, scanner::flag::O_ZEROED};

        fn __events<'de>(mut parser: ParseIter<'de>) -> anyhow::Result<()>
        {
            $( events!(@unwrap $op parser => $match $(=> $msg)?); )+

            Ok(())
        }

        let reader = BorrowReader::new("");
        let iter = PeekReader::new(Reader::from_parts(
            &reader,
            O_ZEROED,
            $tokens,
            true,
        ));

       if let Err(e) = __events(ParseIter::new(iter)) {
           panic!("events! error: {}", e)
       }

       ()
    }};


    (@unwrap | $parser:expr => $event:expr $(=> $msg:tt)? ) => {
        events!(@token $parser, $event $(=> $msg)? )
    };
    (@unwrap @ $parser:expr => $expected:expr $(=> $msg:tt)? ) => {
        assert_eq!($parser.next().transpose()?, $expected $(, $msg )?)
    };
    (@unwrap > $parser:expr => $expected:expr $(=> $msg:tt)? ) => {
        let result = match $parser.next()
        {
            Some(result) => result,
            None => anyhow::bail!("Unexpected end of events, was expecting: {:?} ~{:?}", $expected, $parser.tokens.queue())
        };

        assert_eq!(result, $expected)
    };

    (@token $parser:expr, $expected:expr) => {
        let event = match $parser.next()
        {
            Some(r) => match r
            {
                Ok(r) => r,
                Err(e) => anyhow::bail!("Expected event {:?} got error: {} ~{:?}", $expected, e, $parser.tokens.queue()),
            }
            None => anyhow::bail!("Unexpected end of events, was expecting: {:?} ~{:?}", $expected, $parser.tokens.queue())
        };

        assert_eq!(event, $expected)
    };
    (@token $parser:expr, $event:expr => $msg:tt) => {
        let event = match $parser.next()
        {
            Some(r) => match r
            {
                Ok(r) => r,
                Err(e) => anyhow::bail!("{}: {:?} got error: {} ~{:?}", $msg, $expected, e, $parser.tokens.queue()),
            }
            None => anyhow::bail!("Unexpected end of events, was expecting: {:?} ~{:?}", $expected, $parser.tokens.queue())
        };

        assert_eq!(event, $expected)
    };
}
