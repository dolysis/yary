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

use crate::{
    scanner::{
        eat_whitespace,
        error::{ScanError, ScanResult as Result},
        scalar::escape::tag_uri_unescape,
    },
    token::{Ref, Token},
};

/// Scan a tag directive from .base returning a tag
/// directive token.
///
/// This function will attempt to borrow from .base where
/// possible, but may also copy the directive's handle and
/// prefix into .scratch if borrowing is not possible.
pub(in crate::scanner) fn scan_tag_directive<'b, 'c>(
    base: &'b str,
    scratch: &'c mut Vec<u8>,
) -> Result<(Ref<'b, 'c>, usize)>
{
    let mut buffer = base;
    let mut can_borrow = true;

    // %TAG !named! :tag:prefix # a comment\n
    //      ^^^^^^^
    let (handle, amt) = match scan_tag_handle(buffer)?
    {
        Some((handle, amt)) => (handle.into_inner(), amt),
        None => return Err(ScanError::InvalidTagHandle),
    };
    advance!(buffer, amt);

    // %TAG !named! :tag:prefix # a comment\n
    //             ^
    // Check that there is >= 1 whitespace between handle and
    // prefix
    isBlank!(~buffer, else ScanError::InvalidTagPrefix)?;

    // Chomp whitespace to prefix
    advance!(buffer, eat_whitespace(buffer, false));

    // %TAG !named! :tag:prefix # a comment\n
    //              ^^^^^^^^^^^
    let (prefix, amt) = scan_tag_uri(buffer, scratch, &mut can_borrow, false)?;

    // %TAG !named! tag-prefix # a comment\n
    //                        ^
    // Check there is whitespace or a newline after the tag
    isWhiteSpace!(~buffer, amt, else ScanError::InvalidTagPrefix)?;

    // If we can borrow, just take the range directly out of
    // .buffer
    let token = if can_borrow
    {
        Token::TagDirective(cow!(handle), cow!(&buffer[prefix])).borrowed()
    }
    // Otherwise, we'll need to copy both the handle and prefix, to unify our
    // lifetimes. Note that this isn't strictly necessary, but requiring Token to
    // contain two unrelated lifetimes is just asking for pain and suffering.
    else
    {
        let start = scratch.len();
        scratch.extend_from_slice(handle.as_bytes());

        let handle = std::str::from_utf8(&scratch[start..]).unwrap();
        let prefix = std::str::from_utf8(&scratch[prefix]).unwrap();

        Token::TagDirective(cow!(handle), cow!(prefix)).copied()
    };

    advance!(buffer, amt);

    Ok((token, base.len() - buffer.len()))
}

/// Scan a node's tag, returning a [Token::Tag][Token] with
/// the tag's handle and suffix.
///
/// This function will attempt to borrow from .base, but may
/// copy the handle and suffix into .scratch if borrowing is
/// impossible.
///
/// Furthermore, this function returns several distinct tag
/// patterns (handle, suffix):
///
/// ("", suffix) => A verbatim tag
/// ("!", "") => A non resolving tag
/// (handle, suffix) => A primary, secondary or named tag
pub(in crate::scanner) fn scan_node_tag<'b, 'c>(
    base: &'b str,
    scratch: &'c mut Vec<u8>,
) -> Result<(Ref<'b, 'c>, usize)>
{
    let mut buffer = base;
    let mut can_borrow = true;

    /*
     * Note that the odd &buffer[0..0] is intentional.
     *
     * Certain crates (looking at you Bytes) abuse Rust's
     * pointer semantics to determine if a pointer is
     * "inside" another pointer, and str literals would
     * violate the "inside"ness, so instead we slice
     * a zero length sub-slice out.
     */

    // !<global:verbatim:tag:> "node"
    // ^^
    // If its a verbatim tag scan it
    let (token, amt) = if check!(~buffer => [b'!', b'<', ..])
    {
        advance!(buffer, 2);

        // !<global:verbatim:tag:> "node"
        //   ^^^^^^^^^^^^^^^^^^^^
        let (verbatim, amt) = scan_tag_uri(buffer, scratch, &mut can_borrow, true)?;

        // !<global:verbatim:tag:> "node"
        //                       ^
        check!(~buffer, amt + 1 => b'>', else ScanError::InvalidTagSuffix)?;

        let token = assemble_tag(buffer, scratch, &buffer[0..0], verbatim, can_borrow);

        (token, amt + 1)
    }
    // Otherwise scan it as a normal tag
    else
    {
        match scan_tag_handle(buffer)?
        {
            // ! "node"
            // ^
            // Single ! without a suffix disables tag resolution
            Some((TagHandle::Primary(h), amt)) =>
            {
                (Token::Tag(cow!(h), cow!(&buffer[0..0])).borrowed(), amt)
            },
            // !!global "node" OR !named!global "node"
            // ^^                 ^^^^^^^
            // Got a secondary or named tag, scan the suffix now
            Some((TagHandle::Secondary(h), amt)) | Some((TagHandle::Named(h), amt)) =>
            {
                advance!(buffer, amt);

                // !!global "node" OR !named!global "node"
                //   ^^^^^^                  ^^^^^^
                let (suffix, amt) = scan_tag_uri(buffer, scratch, &mut can_borrow, false)?;

                let token = assemble_tag(buffer, scratch, h, suffix, can_borrow);

                (token, amt)
            },
            // !local "node"
            // Handle scan couldn't find a closing !, meaning this is a local tag
            None =>
            {
                // !local "node"
                // ^
                let handle = &buffer[..1];
                advance!(buffer, 1);

                // !local "node"
                //  ^^^^^
                let (suffix, amt) = scan_tag_uri(buffer, scratch, &mut can_borrow, false)?;

                let token = assemble_tag(buffer, scratch, handle, suffix, can_borrow);

                (token, amt)
            },
        }
    };

    advance!(buffer, amt);

    Ok((token, base.len() - buffer.len()))
}

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

/// Helper function for constructing
/// [Ref][Ref]<[Token::Tag][Token]>s
fn assemble_tag<'b, 'c>(
    buffer: &'b str,
    scratch: &'c mut Vec<u8>,
    handle: &'b str,
    suffix: Range<usize>,
    can_borrow: bool,
) -> Ref<'b, 'c>
{
    if can_borrow
    {
        Token::Tag(cow!(handle), cow!(&buffer[suffix])).borrowed()
    }
    else
    {
        let start = scratch.len();
        scratch.extend_from_slice(handle.as_bytes());

        let h = std::str::from_utf8(&scratch[start..]).unwrap();
        let t = std::str::from_utf8(&scratch[suffix]).unwrap();

        Token::Tag(cow!(h), cow!(t)).copied()
    }
}
