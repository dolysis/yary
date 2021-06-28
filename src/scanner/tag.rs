//! This module contains functions and helpers for scanning
//! YAML tags.
//!
//! A quick refresher on the terminology used in this
//! module. In YAML, a tag is made out of several
//! components:
//!
//! 1. handle
//! 2. prefix
//! 3. suffix
//!
//! Every "resolved" tag has each component.
//!
//! ### A visual aid
//!
//! ---
//! %TAG <handle> <prefix>
//! key: <handle><suffix> value
//! ...
//!
//! ### Handle
//!
//! A handle is one of: !, !! or !:alphanumeric:!, each of
//! which is referred to as primary, secondary and named
//! respectively.
//!
//! ### Prefix
//!
//! A prefix is defined by a %TAG directive, with the
//! primary and secondary handles (! and !!)
//! having override-able defaults:
//!
//! - !  => '!'
//! - !! => 'tag:yaml.org,2002:'
//!
//! No the primary's definition is not recursive; any prefix
//! which starts with a '!' is considered a local tag which
//! has different semantics to global tags, which start with
//! any other character.
//!
//! ### Suffix
//!
//! A suffix is defined on a YAML node and is concated with
//! the resolved prefix, for example '!!str' becomes
//! 'tag:yaml.org,2002:str', '!my_local_tag' becomes
//! '!my_local_tag' -- note that the ! in the resolved tag
//! _does not_ mean the same thing as the ! in the tag.
//!
//! ### An exception to the rule
//!
//! Note above that every "resolved" tag has all three
//! components. YAML allows users to disable tag resolution
//! on scalar nodes (and only scalar nodes) by placing a
//! single '!' without a suffix (e.g 'key: ! "my value"'),
//! which forces the parser to resolve the tag based on the
//! node type. In this case, the tag has no suffix or prefix
//! and is effectively null.
//!
//! ### An aside
//!
//! Can I just say what a gigantic mess it is to use the
//! same character ('!') to mean three different things
//! depending on the context. What a massive headache.

use std::ops::Range;

use crate::scanner::{
    error::{ScanError, ScanResult as Result},
    scalar::escape::tag_uri_unescape,
};

/// Scan a tag directive prefix, as defined in
/// [Section 6.22][Link], returning a range from either
/// .base, or .scratch (if a copy was required), and the
/// amount read from .base. It is the caller's
/// responsibility to check .can_borrow for whether to range
/// into .base or .scratch.
///
/// [Link]: https://yaml.org/spec/1.2/spec.html#ns-global-tag-prefix
pub(in crate::scanner) fn scan_tag_uri(
    base: &str,
    scratch: &mut Vec<u8>,
    can_borrow: &mut bool,
    verbatim: bool,
) -> Result<(Range<usize>, usize)>
{
    let mut buffer = base;
    let start = scratch.len();

    loop
    {
        match buffer.as_bytes()
        {
            // If its a normal allowed character, add it
            [b'&'..=b'+', ..]   // & ' ( ) * +
            | [b'-'..=b'9', ..] // - . / 0-9
            | [b'A'..=b'Z', ..]
            | [b'a'..=b'z', ..]
            | [b'!', ..]
            | [b'$', ..]
            | [b':', ..]
            | [b';', ..]
            | [b'=', ..]
            | [b'?', ..]
            | [b'@', ..]
            | [b'_', ..]
            | [b'~', ..] =>
            {
                if !*can_borrow
                {
                    scratch.push(buffer.as_bytes()[0]);
                }
                advance!(buffer, 1);
            },
            // Further characters are allowed in verbatim tags
            [b',', ..] | [b'[', ..] | [b']', ..] if verbatim =>
            {
                if !*can_borrow
                {
                    scratch.push(buffer.as_bytes()[0]);
                }
                advance!(buffer, 1);
            },
            // If its an escape sequence, we must copy
            [b'%', ..] =>
            {
                if *can_borrow
                {
                    // Safety: we will be indexing to _at most_ base's length
                    scratch.extend_from_slice(&base.as_bytes()[..base.len() - buffer.len()]);

                    *can_borrow = false;
                }
                let amt = tag_uri_unescape(buffer, scratch, true)?;
                advance!(buffer, amt);
            },
            // EOF before loop end is an error
            [] => return Err(ScanError::UnexpectedEOF),
            // Otherwise we've finished the tag, exit the loop
            _ => break,
        }
    }

    let advance = base.len() - buffer.len();

    if *can_borrow
    {
        Ok((0..advance, advance))
    }
    else
    {
        Ok((start..scratch.len(), advance))
    }
}

/// Scans a tag handle from .base, attempting to return the
/// fragment if the handle is unambiguous.
pub(in crate::scanner) fn scan_tag_handle(base: &str) -> Result<Option<(TagHandle, usize)>>
{
    let buffer = base;

    // %TAG !handle! tag-prefix # a comment \n
    //      ^
    // !!tag
    // ^
    // Check that we are indeed starting a handle
    check!(~buffer => b'!', else ScanError::InvalidTagHandle)?;

    // %TAG !handle! tag-prefix # a comment \n
    //       ^^^^^^
    // !handle!tag
    //  ^^^^^^
    // Safety: we just proved above we have >= 1 byte ('!')
    let name = take_while(buffer[1..].as_bytes(), u8::is_ascii_alphanumeric);
    let mut offset = 1 + name.len();

    match buffer.as_bytes().get(offset)
    {
        // If we find a closing '!', then it must either be a secondary or named handle
        Some(b'!') =>
        {
            offset += 1;
            let handle = match name.len()
            {
                0 => (TagHandle::Secondary(&buffer[..2]), 2),
                _ => (TagHandle::Named(&buffer[..offset]), offset),
            };

            Ok(Some(handle))
        },
        // Else check to see if this could be a primary handle (or non-resolving tag)
        _ if name.is_empty() && isWhiteSpace!(~buffer, offset) =>
        {
            let handle = (TagHandle::Primary(&buffer[..1]), 1);

            Ok(Some(handle))
        },
        // Ambiguous sequence, we cannot determine if this is an error, or a local tag + suffix e.g
        // key: !suffix "value"
        //      ^^^^^^^
        Some(_) => Ok(None),
        None => Err(ScanError::UnexpectedEOF),
    }
}

#[derive(Debug)]
pub(in crate::scanner) enum TagHandle<'a>
{
    Primary(&'a str),
    Secondary(&'a str),
    Named(&'a str),
}

impl<'a> TagHandle<'a>
{
    pub fn into_inner(self) -> &'a str
    {
        match self
        {
            Self::Primary(h) | Self::Secondary(h) | Self::Named(h) => h,
        }
    }
}

fn take_while<F>(b: &[u8], f: F) -> &[u8]
where
    F: Fn(&u8) -> bool,
{
    let mut index = 0;

    loop
    {
        match b.get(index)
        {
            Some(b) if f(b) => index += 1,
            _ => return &b[..index],
        }
    }
}
