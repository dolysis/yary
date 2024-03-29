/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! This module contains the functions responsible for
//! scanning block scalars into Tokens.
//!
//! It exports 3 functions:
//!
//! - scan_block_scalar
//! - scan_block_scalar_eager
//! - scan_block_scalar_lazy
//!
//! The eager variant produces a scalar Token (or an error)
//! that may allocate and performs any processing the YAML
//! spec requires. The lazy variant instead defers any
//! processing, returning a structure that can process the
//! scalar at a later time.
//!
//! scan_block_scalar provides the top level interface of
//! this functionality.
//!
//! Two further functions are notable: scan_indent and
//! scan_chomp.
//!
//! scan_indent handles the scanning of scalar indentation,
//! and will typically be called once per line in a block
//! scalar. It is also indirectly responsible for
//! terminating the main loop which relies on the local
//! indent level that scan_indent sets.
//!
//! scan_chomp finishes the scalar scanning, and is
//! responsible for ensuring the correct amount of
//! trailing whitespace is added to the scalar based on its
//! chomp header -- the '|' or '>'.

use std::num::NonZeroU8;

use atoi::atoi;

use crate::{
    scanner::{
        context::Context,
        entry::MaybeToken,
        error::{ScanError, ScanResult as Result},
        flag::{Flags, O_LAZY},
        scalar::as_maybe,
        stats::MStats,
    },
    token::{ScalarStyle, Slice, Token},
};

/// Scans a block scalar returning an opaque handle to a
/// byte slice that could be a valid scalar.
///
/// This function is a wrapper around
/// scan_block_scalar_eager and scan_block_scalar_lazy. See
/// the respective documentation for an explanation.
pub(in crate::scanner) fn scan_block_scalar<'de>(
    opts: Flags,
    base: &'de str,
    stats: &mut MStats,
    cxt: &Context,
    fold: bool,
) -> Result<(MaybeToken<'de>, usize)>
{
    // It is safe to assume our indentation level is >=0 here
    // because block scalars can only occur as mapping or
    // sequence values
    let indent = cxt.indent().as_usize();

    match opts.contains(O_LAZY)
    {
        true => scan_block_scalar_lazy(opts, base, stats, indent, fold).map(as_maybe),
        false => scan_block_scalar_eager(opts, base, stats, indent, fold).map(as_maybe),
    }
}
/// Scans a block scalar, returning a Token and the amount
/// read from .base. This function will attempt to borrow
/// from .base, however the circumstances in which it
/// remains possible to borrow are narrow.
///
/// See:
///     YAML 1.2: Section 8.1
///     yaml.org/spec/1.2/#c-b-block-header(m,t)
pub(in crate::scanner) fn scan_block_scalar_eager<'de>(
    opts: Flags,
    base: &'de str,
    stats: &mut MStats,
    base_indent: usize,
    fold: bool,
) -> Result<(Token<'de>, usize)>
{
    // Initialize the local state handlers
    let mut buffer = base;
    let mut scratch = Vec::new();
    let mut local_stats = stats.clone();

    // Tracks if a borrow is possible from the underlying .base
    let mut can_borrow = true;

    // Tracks the start and end of the scalar content. Note that
    // these track two different values depending on whether
    // we .can_borrow. If we can, acts as start/end indexes
    // into .base, otherwise as start/end indexes into
    // .scratch. These can be difficult to keep track of; so
    // pay attention to the context before setting them.
    let mut content_start: usize = 0;
    let mut content_end: usize = 0;

    // Keeps track of the outstanding lines that need to be
    // reconciled
    let mut lines: usize = 0;

    // The indentation level of this scalar
    let indent: usize;

    // Scalar style mapping
    let style = match fold
    {
        true => ScalarStyle::Folded,
        false => ScalarStyle::Literal,
    };

    // Eat the '|' or '>'
    cache!(~buffer, 1, opts)?;
    advance!(buffer, :local_stats, 1);

    // Calculate any headers this scalar may have
    let (chomp, explicit) = scan_headers(opts, &mut buffer, &mut local_stats)?;

    // The header line must contain nothing after the headers
    // excluding a comment until the line ending
    skip_blanks(opts, &mut buffer, &mut local_stats, COMMENTS)?;
    cache!(~buffer, 1, opts)?;
    if !isWhiteSpaceZ!(~buffer)
    {
        return Err(ScanError::InvalidBlockScalar);
    }

    // Eat the line break
    advance!(buffer, :local_stats, @line);

    // Set the indent explicitly if defined, otherwise detect
    // from the indentation level
    match explicit.map(NonZeroU8::get)
    {
        Some(explicit) => indent = base_indent + explicit as usize,
        None =>
        {
            indent = detect_indent_level(
                opts,
                &mut buffer,
                &mut local_stats,
                base_indent,
                &mut lines,
                &mut can_borrow,
            )?
        },
    }

    // Add any preceding lines to the tracked borrow or scratch
    // space
    match can_borrow
    {
        true => content_start = local_stats.read - stats.read,
        false =>
        {
            for _ in 0..lines
            {
                scratch.push(NEWLINE)
            }
        },
    }

    lines = 0;

    // Loop over scalar line by line until we reach a less
    // indented line or EOF
    while local_stats.column == indent && (!buffer.is_empty())
    {
        /*
         * We're at the start of an indented line
         */

        // Trapdoor to alloc land. Unfortunately, block scalars are
        // very unforgiving in which cases can be borrowed.
        // Basically limiting us to the unusual case of a single
        // line directly following the header line. E.g:
        //
        //  key: |  # or >
        //    I can be borrowed
        if can_borrow && lines > 0
        {
            scratch.extend_from_slice(base[content_start..content_end].as_bytes());

            can_borrow = false
        }

        // If its a folding ('>') block scalar
        if fold
        {
            // Handle line joins as needed
            match lines
            {
                // No join needed, we're done
                0 =>
                {},
                // If a single line was recorded, we _cannot_ have seen a line wholly made of
                // whitespace, therefore join via a space
                1 =>
                {
                    scratch.push(SPACE);
                },
                // Else we need to append (n - 1) newlines, as we skip the origin line's break
                n =>
                {
                    // Safety: we can only reach this branch if n > 1
                    for _ in 0..n - 1
                    {
                        scratch.push(NEWLINE)
                    }
                },
            }
        }
        // Otherwise simply append the collected newlines literally ('|')
        else
        {
            for _ in 0..lines
            {
                scratch.push(NEWLINE)
            }
        }

        // Reset line counter for next iteration
        lines = 0;

        // Mark content start
        match can_borrow
        {
            true =>
            {
                if content_start == 0
                {
                    content_start = local_stats.read - stats.read
                }
            },
            false => content_start = 0,
        }

        // Eat the line's content until the line break (or EOF)
        cache!(~buffer, 1, opts)?;
        while !isBreakZ!(~buffer)
        {
            cache!(~buffer, 1, opts)?;

            if !can_borrow
            {
                scratch.push(buffer.as_bytes()[0])
            }
            advance!(buffer, :local_stats, 1);
        }

        // Mark content end
        match can_borrow
        {
            true => content_end = local_stats.read - stats.read,
            false => content_end = scratch.len(),
        }

        // Eat the line break (if not EOF)
        cache!(~buffer, 1, opts)?;
        if isBreak!(~buffer)
        {
            advance!(buffer, :local_stats, @line);
            lines += 1;
        }

        // Chomp indentation until the next indented line
        scan_indent(
            opts,
            &mut buffer,
            &mut local_stats,
            &mut lines,
            &mut can_borrow,
            indent,
        )?;
    }

    // Scan the ending whitespace, returning the final scalar
    let c_params = ChompParams::new(chomp, content_start, content_end, lines);
    let scalar = scan_chomp(base, scratch, &mut can_borrow, c_params)?;

    *stats = local_stats;
    let advance = base.len() - buffer.len();
    let token = Token::Scalar(scalar, style);

    Ok((token, advance))
}

pub(in crate::scanner) fn scan_block_scalar_lazy<'de>(
    opts: Flags,
    base: &'de str,
    stats: &mut MStats,
    base_indent: usize,
    fold: bool,
) -> Result<(Deferred<'de>, usize)>
{
    // Initialize the local state handlers
    let mut buffer = base;
    let mut local_stats = stats.clone();

    // The indentation level of this scalar
    let indent: usize;

    // Eat the '|' or '>'
    cache!(~buffer, 1, opts)?;
    advance!(buffer, :local_stats, 1);

    // Calculate any headers this scalar may have
    let (_, explicit) = scan_headers(opts, &mut buffer, &mut local_stats)?;

    // The header line must contain nothing after the headers
    // excluding a comment until the line ending
    skip_blanks(opts, &mut buffer, &mut local_stats, COMMENTS)?;
    cache!(~buffer, 1, opts)?;
    if !isWhiteSpaceZ!(~buffer)
    {
        return Err(ScanError::InvalidBlockScalar);
    }

    // Eat the line break
    advance!(buffer, :local_stats, @line);

    // Set the indent explicitly if defined, otherwise detect
    // from the indentation level
    match explicit.map(NonZeroU8::get)
    {
        Some(explicit) => indent = base_indent + explicit as usize,
        None =>
        {
            indent = detect_indent_level(
                opts,
                &mut buffer,
                &mut local_stats,
                base_indent,
                &mut 0,
                &mut false,
            )?
        },
    }

    while local_stats.column == indent && (!buffer.is_empty())
    {
        /*
         * We're at the start of an indented line
         */

        // Eat the line's content until the line break (or EOF)
        cache!(~buffer, 1, opts)?;
        while !isBreakZ!(~buffer)
        {
            cache!(~buffer, 1, opts)?;
            advance!(buffer, :local_stats, 1);
        }

        // Eat the line break (if not EOF)
        cache!(~buffer, 1, opts)?;
        if isBreak!(~buffer)
        {
            advance!(buffer, :local_stats, @line);
        }

        // Chomp indentation until the next indented line
        scan_indent(
            opts,
            &mut buffer,
            &mut local_stats,
            &mut 0,
            &mut false,
            indent,
        )?;
    }

    let advance = base.len() - buffer.len();
    let slice = &base[..advance];

    let lazy = Deferred::new(opts, slice, stats.clone(), base_indent, fold);

    *stats = local_stats;

    Ok((lazy, advance))
}

/// Retrieve a block scalar's headers
fn scan_headers(
    opts: Flags,
    buffer: &mut &str,
    stats: &mut MStats,
) -> Result<(ChompStyle, IndentHeader)>
{
    let mut skip = 0;
    let mut indent = None;
    let mut chomp = ChompStyle::Clip;

    cache!(~buffer, 2, opts)?;

    // Set the explicit indent if it exists.
    //
    // Note that we silently eat an invalid indent (0) rather
    // than erroring
    match buffer.as_bytes()
    {
        [i @ b'0'..=b'9', ..] | [_, i @ b'0'..=b'9', ..] =>
        {
            indent = atoi::<u8>(&[*i]).and_then(NonZeroU8::new);
            skip += 1;
        },
        _ =>
        {},
    }

    // Set the chomping behavior of the scalar, if required
    match buffer.as_bytes()
    {
        [c, ..] | [_, c, ..] if matches!(*c, b'+') =>
        {
            chomp = ChompStyle::Keep;
            skip += 1;
        },
        [c, ..] | [_, c, ..] if matches!(*c, b'-') =>
        {
            chomp = ChompStyle::Strip;
            skip += 1;
        },
        _ =>
        {},
    }

    advance!(*buffer, :stats, skip);

    Ok((chomp, indent))
}

/// Chomp the indentation spaces of a block scalar
fn scan_indent(
    opts: Flags,
    buffer: &mut &str,
    stats: &mut MStats,
    lines: &mut usize,
    _can_borrow: &mut bool,
    indent: usize,
) -> Result<bool>
{
    if stats.column >= indent
    {
        return Ok(false);
    }

    cache!(~buffer, 1, opts)?;

    while stats.column < indent && isWhiteSpace!(~buffer)
    {
        // Indentation space, chomp
        if check!(~buffer => b' ')
        {
            advance!(*buffer, :stats, 1);
        }
        // Tabs in indentation, error
        else if check!(~buffer => b'\t')
        {
            return Err(ScanError::InvalidTab);
        }
        // Line break, chomp; increment lines
        else if isBreak!(~buffer)
        {
            *lines += 1;
            advance!(*buffer, :stats, @line);
        }

        cache!(~buffer, 1, opts)?;
    }

    Ok(true)
}

/// Process a block scalar's ending whitespace according to
/// the YAML Spec section 8.1.1.2.
///
/// See:
///     yaml.org/spec/1.2/#c-chomping-indicator(t)
fn scan_chomp<'de>(
    base: &'de str,
    mut scratch: Vec<u8>,
    can_borrow: &mut bool,
    params: ChompParams,
) -> Result<Slice<'de>>
{
    let mut scalar = cow!("");
    let ChompParams {
        style,
        start,
        mut end,
        mut lines,
    } = params;

    if *can_borrow
    {
        match style
        {
            // Clip the scalar to 0 or 1 line break
            ChompStyle::Clip =>
            {
                // Check if we had trailing lines, extending the borrow to
                // include the first
                if lines > 0
                {
                    end += widthOf!(~base[end..], 1);
                }

                scalar = cow!(&base[start..end])
            },
            // Ignore any trailing line breaks just returning the scalar
            ChompStyle::Strip => scalar = cow!(&base[start..end]),
            // We only maintain the borrow if there is 0 or 1 new lines
            //
            // Technically, we could extend the logic here to check if the most recent scan_indent
            // didn't skip any spaces (only line breaks).
            ChompStyle::Keep => match lines
            {
                n @ 0 | n @ 1 => scalar = cow!(&base[start..end + n]),
                // The only way to hit this branch is if the scalar could still be borrowed, and
                // thus is a single line. In this case we have to copy the borrow to the scratch
                // space, and append any trailing lines the previous scan_indent produced.
                //
                // Note that .start and .end in this case refer to .base offsets _AND NOT_ scratch
                // offsets. Care must be taken to ensure we _NEVER_ index into scratch with them
                // when .style == Keep
                _ =>
                {
                    scratch.extend_from_slice(base[start..end].as_bytes());

                    while lines > 0
                    {
                        scratch.push(NEWLINE);
                        lines -= 1;
                    }

                    // Ensure we hit the copy branch below so as to correctly
                    // handle .scratch -> .scalar transform
                    *can_borrow = false;
                },
            },
        }
    }

    if !*can_borrow
    {
        match style
        {
            // Clip the trailing line breaks to 0 or 1, appending as necessary
            ChompStyle::Clip =>
            {
                if lines > 0
                {
                    scratch.push(NEWLINE);
                    end += 1;
                }

                scratch.truncate(end);
            },
            // Return the content as is, no trailing whitespace
            ChompStyle::Strip => scratch.truncate(end),
            // Append any trailing line breaks that weren't caught in the main loop of
            // scan_block_scalar
            ChompStyle::Keep =>
            {
                for _ in 0..lines
                {
                    scratch.push(NEWLINE)
                }
            },
        }

        scalar = cow!(String::from_utf8(scratch).unwrap());
    }

    Ok(scalar)
}

/// Auto-detect the indentation level from the first non
/// header line of a block scalar
fn detect_indent_level(
    opts: Flags,
    buffer: &mut &str,
    stats: &mut MStats,
    base_indent: usize,
    lines: &mut usize,
    can_borrow: &mut bool,
) -> Result<usize>
{
    let mut indent = 0;

    loop
    {
        cache!(~buffer, 1, opts)?;

        // Chomp indentation spaces, erroring on a tab
        while isBlank!(~buffer)
        {
            cache!(~buffer, 1, opts)?;

            if check!(~buffer => b'\t')
            {
                return Err(ScanError::InvalidTab);
            }

            if *can_borrow && *lines > 0
            {
                *can_borrow = false
            }

            advance!(*buffer, :stats, 1);
        }

        // Update detected indentation if required
        if stats.column > indent
        {
            indent = stats.column;
        }

        // If its not a line break we're done, exit the loop
        cache!(~buffer, 1, opts)?;
        if !isBreak!(~buffer)
        {
            break;
        }

        // Otherwise eat the line and repeat
        advance!(*buffer, :stats, @line);
        *lines += 1;
    }

    // Note that we must set the lower bound of the indentation
    // level, in case the YAML is invalid
    if indent < base_indent + 1
    {
        indent = base_indent + 1
    }

    Ok(indent)
}

/// Skip any blanks (and .comments) until we reach a line
/// ending or non blank character
fn skip_blanks(opts: Flags, buffer: &mut &str, stats: &mut MStats, comments: bool) -> Result<()>
{
    cache!(~buffer, 1, opts)?;

    while isBlank!(~buffer)
    {
        cache!(~buffer, 1, opts)?;
        advance!(*buffer, :stats, 1);
    }

    if comments && check!(~buffer => b'#')
    {
        while !isBreakZ!(~buffer)
        {
            cache!(~buffer, 1, opts)?;
            advance!(*buffer, :stats, 1);
        }
    }

    Ok(())
}

/// The type of chomping associated with a block scalar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChompStyle
{
    /// Clipping is the default behavior used if no explicit
    /// chomping indicator is specified. In this case, the
    /// final line break character is preserved in the
    /// scalar’s content. However, any trailing empty lines
    /// are excluded from the scalar’s content.
    Clip,
    /// Stripping is specified by the '-' chomping
    /// indicator. In this case, the final line break and
    /// any trailing empty lines are excluded from the
    /// scalar’s content.
    Strip,
    /// Keeping is specified by the “+” chomping indicator.
    /// In this case, the final line break and any trailing
    /// empty lines are considered to be part of the
    /// scalar’s content. These additional lines are not
    /// subject to folding.
    Keep,
}

impl Default for ChompStyle
{
    fn default() -> Self
    {
        Self::Clip
    }
}

/// Packager for transporting args into scan_chomp without
/// triggering clippy
#[derive(Debug)]
struct ChompParams
{
    pub style: ChompStyle,
    pub start: usize,
    pub end:   usize,
    pub lines: usize,
}

impl ChompParams
{
    fn new(style: ChompStyle, start: usize, end: usize, lines: usize) -> Self
    {
        Self {
            style,
            start,
            end,
            lines,
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::scanner) struct Deferred<'de>
{
    opts:   Flags,
    slice:  &'de str,
    stats:  MStats,
    indent: usize,
    fold:   bool,
}

impl<'de> Deferred<'de>
{
    pub fn new(opts: Flags, slice: &'de str, stats: MStats, indent: usize, fold: bool) -> Self
    {
        Self {
            opts,
            slice,
            stats,
            indent,
            fold,
        }
    }

    pub fn into_token(self) -> Result<Token<'de>>
    {
        let Deferred {
            opts,
            slice,
            mut stats,
            indent,
            fold,
        } = self;

        scan_block_scalar_eager(opts, slice, &mut stats, indent, fold).map(|(t, _)| t)
    }
}

/// Indentation level explicitly set in a block scalar's
/// headers
type IndentHeader = Option<NonZeroU8>;

const COMMENTS: bool = true;
const SPACE: u8 = b' ';
const NEWLINE: u8 = b'\n';

#[cfg(test)]
mod tests
{
    use pretty_assertions::assert_eq;
    use ScalarStyle::{Folded, Literal};

    use super::*;
    use crate::scanner::scalar::test_utils::{normalize, TestResult, TEST_FLAGS};

    macro_rules! cxt {
        (flow -> $level:expr) => {
            {
                let mut c = Context::new();

                for _ in 0..$level {
                    c.flow_increment().unwrap();
                }

                c
            }
        };
        (block -> [ $($indent:expr),+ ]) => {
            {
                let mut c = Context::new();
                $( cxt!(@blk &mut c, $indent) )+;

                c
            }
        };
        (@blk $cxt:expr, $indent:expr) => {
            $cxt.indent_increment($indent, 0, true).unwrap()
        }
    }

    /* === LITERAL STYLE === */

    #[test]
    fn literal_simple() -> TestResult
    {
        let data = "|\n  this is a simple block scalar";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("this is a simple block scalar"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_clip() -> TestResult
    {
        let data = "|\n  trailing lines...\n \n\n";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("trailing lines...\n"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_strip() -> TestResult
    {
        let data = "|-\n  trailing lines...\n \n\n";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("trailing lines..."), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_keep() -> TestResult
    {
        let data = "|+\n  trailing lines...\n \n\n";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("trailing lines...\n\n\n"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_line_folding() -> TestResult
    {
        let data = "|
  some folded
  lines
  here
";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("some folded\nlines\nhere\n"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_preceding_breaks() -> TestResult
    {
        let data = "|-


  some folded
  lines
  here
";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("\n\nsome folded\nlines\nhere"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_trailing_breaks() -> TestResult
    {
        let data = "|+
  some folded
  lines
  here


";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("some folded\nlines\nhere\n\n\n"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_trailing_chars() -> TestResult
    {
        let data = "|+
  some folded
  lines
  here


some.other.key: value";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("some folded\nlines\nhere\n\n\n"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_interior_breaks() -> TestResult
    {
        let data = "|-
  this

  has

  breaks
";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("this\n\nhas\n\nbreaks"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn literal_comment() -> TestResult
    {
        let data = "| # a comment here.\n  simple block scalar";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("simple block scalar"), Literal);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    /* === FOLDED STYLE === */

    #[test]
    fn folded_simple() -> TestResult
    {
        let data = ">\n  this is a simple block scalar";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("this is a simple block scalar"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_clip() -> TestResult
    {
        let data = ">\n  trailing lines...\n \n\n";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("trailing lines...\n"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_strip() -> TestResult
    {
        let data = ">-\n  trailing lines...\n \n\n";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("trailing lines..."), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_keep() -> TestResult
    {
        let data = ">+\n  trailing lines...\n \n\n";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("trailing lines...\n\n\n"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_line_folding() -> TestResult
    {
        let data = ">
  some folded
  lines
  here
";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("some folded lines here\n"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_preceding_breaks() -> TestResult
    {
        let data = ">-


  some folded
  lines
  here
";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("\n\nsome folded lines here"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_trailing_breaks() -> TestResult
    {
        let data = ">+
  some folded
  lines
  here


";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("some folded lines here\n\n\n"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_trailing_chars() -> TestResult
    {
        let data = ">+
  some folded
  lines
  here


some.other.key: value";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("some folded lines here\n\n\n"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_interior_breaks() -> TestResult
    {
        let data = ">-
  this

  has

  breaks
";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("this\nhas\nbreaks"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    #[test]
    fn folded_comment() -> TestResult
    {
        let data = "> # a comment here.\n  simple block scalar";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("simple block scalar"), Folded);

        let (token, _amt) =
            scan_block_scalar(TEST_FLAGS, data, &mut stats, &cxt, !LITERAL).and_then(normalize)?;

        assert_eq!(token, expected);

        Ok(())
    }

    const LITERAL: bool = false;
}
