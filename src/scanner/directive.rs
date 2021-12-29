/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use atoi::atoi;

use super::{
    error::{ScanError, ScanResult as Result},
    stats::MStats,
};
use crate::{
    scanner::{eat_whitespace, flag::Flags, tag::scan_tag_directive, COMMENTS},
    token::Token,
};

/// Scans a version or tag directive from .buffer, based on
/// the .kind of directive, returning the relevant Token.
pub(in crate::scanner) fn scan_directive<'de>(
    opts: Flags,
    buffer: &mut &'de str,
    stats: &mut MStats,
    kind: &DirectiveKind,
) -> Result<Token<'de>>
{
    match kind
    {
        DirectiveKind::Version =>
        {
            // Chomp any preceding whitespace
            advance!(*buffer, eat_whitespace(opts, buffer, stats, !COMMENTS)?);

            // %YAML 1.1
            //       ^
            let (major, skip) = scan_directive_version(opts, buffer)?;
            advance!(*buffer, :stats, skip);

            // %YAML 1.1
            //        ^
            cache!(~buffer, 1, opts)?;
            check!(~buffer => b'.', else ScanError::InvalidVersion)?;
            advance!(*buffer, :stats, 1);

            // %YAML 1.1
            //         ^
            let (minor, skip) = scan_directive_version(opts, buffer)?;
            advance!(*buffer, :stats, skip);

            Ok(Token::VersionDirective(major, minor))
        },
        DirectiveKind::Tag =>
        {
            // Chomp any spaces up to the handle
            advance!(*buffer, eat_whitespace(opts, buffer, stats, !COMMENTS)?);

            // Scan the directive, copying if necessary
            let (token, amt) = scan_tag_directive(opts, buffer, stats)?;
            advance!(*buffer, amt);

            Ok(token)
        },
    }
}

/// Representation of a YAML directive, either version
/// (%YAML) or tag (%TAG)
pub(in crate::scanner) enum DirectiveKind
{
    Version,
    Tag,
}

impl DirectiveKind
{
    const KIND_VERSION: &'static str = "YAML";
    const KIND_TAG: &'static str = "TAG";

    /// Fallibly determine which (if any) directive starts
    /// the given .buffer
    pub fn new(buffer: &str) -> Result<Self>
    {
        if buffer.starts_with(Self::KIND_VERSION)
        {
            Ok(Self::Version)
        }
        else if buffer.starts_with(Self::KIND_TAG)
        {
            Ok(Self::Tag)
        }
        else
        {
            Err(ScanError::UnknownDirective)
        }
    }

    /// The number of bytes associated with the directive
    pub fn len(&self) -> usize
    {
        match self
        {
            Self::Version => Self::KIND_VERSION.len(),
            Self::Tag => Self::KIND_TAG.len(),
        }
    }
}

fn scan_directive_version(opts: Flags, b: &str) -> Result<(u8, usize)>
{
    let v_slice = take_while(opts, b.as_bytes(), u8::is_ascii_digit)?;
    let v = atoi(v_slice).ok_or(ScanError::InvalidVersion)?;

    Ok((v, v_slice.len()))
}

fn take_while<F>(opts: Flags, base: &[u8], f: F) -> Result<&[u8]>
where
    F: Fn(&u8) -> bool,
{
    let mut index = 0;

    loop
    {
        let i = cache!(base, @index, 1, opts)?;

        match base.get(index)
        {
            Some(b) if f(b) => index += i,
            _ => return Ok(&base[..index]),
        }
    }
}
