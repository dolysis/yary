use std::ops::Range;

use crate::{
    scanner::{
        error::{ScanError, ScanResult as Result},
        scalar::escape::flow_unescape,
        MStats,
    },
    token::{ScalarStyle, Token},
};

/// Scans a single or double quoted (flow) scalar returning
/// a Token containing the contents, and the amount read
/// from .base. This function will attempt to borrow from
/// the underlying .base, however it may be required to copy
/// into .scratch and borrow from that lifetime.
pub(in crate::scanner) fn scan_flow_scalar(
    base: &str,
    stats: &mut MStats,
    single: bool,
) -> Result<(ScalarRange, usize)>
{
    use ScalarStyle::{DoubleQuote, SingleQuote};

    let mut scratch = Vec::new();
    let mut buffer = base;
    let mut can_borrow = true;
    let mut escaped_break;
    let mut whitespace: usize;
    let mut lines: usize;
    let kind = match single
    {
        true => SingleQuote,
        false => DoubleQuote,
    };

    // Eat left quote
    advance!(buffer, :stats, 1);

    'scalar: loop
    {
        escaped_break = None;

        // Even in a scalar context, YAML prohibits starting a line
        // with document stream tokens followed by a blank
        // character
        if isDocumentIndicator!(~buffer, :stats)
        {
            return Err(ScanError::InvalidFlowScalar);
        }

        // EOF without a quote is an error
        if buffer.is_empty()
        {
            return Err(ScanError::UnexpectedEOF);
        }

        // Consume non whitespace characters
        while !isWhiteSpaceZ!(~buffer)
        {
            // if we encounter an escaped quote we can no longer borrow
            // from .base, we must unescape the quote into .scratch
            if kind == SingleQuote && check!(~buffer => [SINGLE, SINGLE, ..])
            {
                set_no_borrow(&mut can_borrow, base, buffer, &mut scratch);

                scratch.push(SINGLE);
                advance!(buffer, :stats, 2);
            }
            // We're done, we hit the right quote
            else if (kind == SingleQuote && check!(~buffer => [SINGLE, ..]))
                || (kind == DoubleQuote && check!(~buffer => [DOUBLE, ..]))
            {
                break 'scalar;
            }
            // We're going to hit an escaped newline, prep the whitespace loop
            else if kind == DoubleQuote
                && check!(~buffer => [BACKSLASH, ..])
                && isBreak!(~buffer, 1)
            {
                set_no_borrow(&mut can_borrow, base, buffer, &mut scratch);

                escaped_break = Some(EscapeState::Start);
                advance!(buffer, :stats, 1);
            }
            // We've hit an escape sequence, parse it
            else if kind == DoubleQuote && check!(~buffer => [BACKSLASH, ..])
            {
                set_no_borrow(&mut can_borrow, base, buffer, &mut scratch);

                let read = flow_unescape(buffer, &mut scratch)?;
                advance!(buffer, :stats, read);
            }
            // Its a non blank character, add it
            else
            {
                if !can_borrow
                {
                    // Safety: !isBlankZ guarantees the slice is not empty
                    scratch.push(buffer.as_bytes()[0])
                }
                advance!(buffer, :stats, 1);
            }
        }

        whitespace = 0;
        lines = 0;

        #[rustfmt::skip]
        /*
         * The YAML spec goes over the rules for quoted scalar line joining in Section
         * 7.3.1 and 7.3.2. In short, on hitting a LINEBREAK, discard all trailing
         * whitespace on the current line, discard any leading whitespace on the next
         * line and if a non WHITESPACE character exists on the next line, append a space
         * (\x20) else append a newline (\x0A).
         *
         * The rules change slightly for escaped line breaks in double quoted scalars,
         * that is the character sequence: [\, LINEBREAK]. In this case, we keep any
         * trailing whitespace, still discard leading whitespace, do not append a
         * space, but still append newline if required.
         *
         * yaml.org/spec/1.2/spec.html#style/flow/double-quoted
         */
        let _ = ();

        // Consume whitespace
        loop
        {
            match (isBlank!(~buffer), isBreak!(~buffer))
            {
                // No more whitespace, exit loop
                (false, false) => break,
                // Handle blanks
                (true, _) =>
                {
                    if !can_borrow
                    {
                        scratch.push(buffer.as_bytes()[0]);
                    }
                    whitespace += 1;
                    advance!(buffer, :stats, 1);
                },
                // Handle line breaks
                (false, _) =>
                {
                    set_no_borrow(&mut can_borrow, base, buffer, &mut scratch);

                    if let Some(EscapeState::Start) = escaped_break
                    {
                        // Reset .whitespace as we keep trailing whitespace for
                        // escaped line breaks
                        whitespace = 0;
                        escaped_break = Some(EscapeState::Started)
                    }

                    lines += 1;
                    advance!(buffer, :stats, @line);
                },
            }
        }

        // Check if we need to handle a line join
        match lines
        {
            // No join needed, we're done
            0 =>
            {},
            // If a single line was recorded, we _cannot_ have seen a line wholly made of
            // whitespace, therefore join via a space
            1 =>
            {
                set_no_borrow(&mut can_borrow, base, buffer, &mut scratch);

                scratch.truncate(scratch.len() - whitespace);

                if escaped_break.is_none()
                {
                    scratch.push(SPACE);
                }
            },
            // Else we need to append (n - 1) newlines, as we skip the origin line's break
            n =>
            {
                set_no_borrow(&mut can_borrow, base, buffer, &mut scratch);

                scratch.truncate(scratch.len() - whitespace);

                // Safety: we can only reach this branch if n > 1
                for _ in 0..n - 1
                {
                    scratch.push(NEWLINE)
                }
            },
        }
    }

    let token: ScalarRange = match can_borrow
    {
        true => (1..base.len() - buffer.len(), kind).into(),
        false =>
        {
            let utf8 = String::from_utf8(scratch).unwrap();

            (utf8, kind).into()
        },
    };

    // Eat the right quote
    advance!(buffer, :stats, 1);

    let advance = base.len() - buffer.len();

    Ok((token, advance))
}

// Handles the trap door from borrowing to copying
fn set_no_borrow(can_borrow: &mut bool, base: &str, buffer: &str, scratch: &mut Vec<u8>)
{
    if *can_borrow
    {
        // Note we start from 1 here to account for the quote
        // character
        scratch.extend_from_slice(base[1..base.len() - buffer.len()].as_bytes());
    }

    *can_borrow = false
}

#[derive(Debug, Clone)]
pub(in crate::scanner) struct ScalarRange
{
    inner: ScalarRangeInner,
    style: ScalarStyle,
}

#[derive(Debug, Clone)]
enum ScalarRangeInner
{
    Borrow(Range<usize>),
    Owned(String),
}

impl ScalarRange
{
    pub fn into_token(self, base: &str) -> Result<Token<'_>>
    {
        use ScalarRangeInner::*;

        match self.inner
        {
            Borrow(range) => Ok(Token::Scalar(cow!(&base[range]), self.style.clone())),
            Owned(s) => Ok(Token::Scalar(cow!(s), self.style.clone())),
        }
    }
}

impl From<(Range<usize>, ScalarStyle)> for ScalarRange
{
    fn from((range, style): (Range<usize>, ScalarStyle)) -> Self
    {
        Self {
            inner: ScalarRangeInner::Borrow(range),
            style,
        }
    }
}

impl From<(String, ScalarStyle)> for ScalarRange
{
    fn from((s, style): (String, ScalarStyle)) -> Self
    {
        Self {
            inner: ScalarRangeInner::Owned(s),
            style,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum EscapeState
{
    Start,
    Started,
}

const SINGLE: u8 = b'\'';
const DOUBLE: u8 = b'"';
const SPACE: u8 = b' ';
const NEWLINE: u8 = b'\n';
const BACKSLASH: u8 = b'\\';

#[cfg(test)]
mod tests
{
    use anyhow::bail;
    use pretty_assertions::assert_eq;

    use super::*;

    type TestResult = anyhow::Result<()>;

    /* ====== SINGLE QUOTED TESTS ====== */

    #[test]
    fn flow_single_empty() -> TestResult
    {
        let data = "''";
        let stats = &mut MStats::new();
        let expected = Token::Scalar(cow!(""), ScalarStyle::SingleQuote);

        let (range, read) = scan_flow_scalar(data, stats, true)?;
        let scalar = range.into_token(data)?;

        assert_eq!(read, 2);

        if !(scalar == expected)
        {
            bail!("expected\n{:?}\nbut got\n{:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_single_simple() -> TestResult
    {
        let data = "'hello world'";
        let stats = &mut MStats::new();
        let expected = Token::Scalar(cow!("hello world"), ScalarStyle::SingleQuote);

        let (range, read) = scan_flow_scalar(data, stats, true)?;
        let scalar = range.into_token(data)?;

        assert_eq!(read, 13);

        if !(scalar == expected)
        {
            bail!("expected\n{:?}\nbut got\n{:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_single_fold_lines() -> TestResult
    {
        let data = r#"'first
            second
            third
fourth'"#;
        let stats = &mut MStats::new();
        let cmp = "first second third fourth";
        let expected = Token::Scalar(cow!(cmp), ScalarStyle::SingleQuote);

        let (range, _read) = scan_flow_scalar(data, stats, true)?;
        let scalar = range.into_token(data)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_single_trim_whitespace() -> TestResult
    {
        let data = r#"'first     
            second'"#;
        let stats = &mut MStats::new();
        let cmp = "first second";
        let expected = Token::Scalar(cow!(cmp), ScalarStyle::SingleQuote);

        let (range, _read) = scan_flow_scalar(data, stats, true)?;
        let scalar = range.into_token(data)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_single_fold_newline() -> TestResult
    {
        let data = r#"'first
            second
        third

            fourth'"#;
        let stats = &mut MStats::new();
        let cmp = "first second third\nfourth";
        let expected = Token::Scalar(cow!(cmp), ScalarStyle::SingleQuote);

        let (range, _read) = scan_flow_scalar(data, stats, true)?;
        let scalar = range.into_token(data)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_single_reject_document()
    {
        let data = ["'\n--- '", "'\n---\n'"];
        let expected = ScanError::InvalidFlowScalar;
        let mut stats;

        for (i, &t) in (&data).into_iter().enumerate()
        {
            stats = MStats::new();

            match scan_flow_scalar(t, &mut stats, true)
            {
                Err(e) => assert_eq!(
                    e, expected,
                    "on iteration {}, expected error {}, got {}",
                    i, expected, e
                ),
                Ok(unexpected) => panic!(
                    "on iteration {}, expected error {}, got unexpected value {:?}",
                    i, expected, unexpected
                ),
            }
        }
    }

    #[test]
    fn flow_single_reject_eof()
    {
        let data = ["'end space ", "'", "'end word"];
        let expected = ScanError::UnexpectedEOF;
        let mut stats;

        for (i, &t) in (&data).into_iter().enumerate()
        {
            stats = MStats::new();

            match scan_flow_scalar(t, &mut stats, true)
            {
                Err(e) => assert_eq!(
                    e, expected,
                    "on iteration {}, expected error {}, got {}",
                    i, expected, e
                ),
                Ok(unexpected) => panic!(
                    "on iteration {}, expected error {}, got unexpected value {:?}",
                    i, expected, unexpected
                ),
            }
        }
    }

    /* ====== DOUBLE QUOTED TESTS ====== */

    #[test]
    fn flow_double_empty() -> TestResult
    {
        let data = r#""""#;
        let stats = &mut MStats::new();
        let expected = Token::Scalar(cow!(""), ScalarStyle::DoubleQuote);

        let (range, read) = scan_flow_scalar(data, stats, false)?;
        let scalar = range.into_token(data)?;

        assert_eq!(read, 2);

        if !(scalar == expected)
        {
            bail!("expected\n{:?}\nbut got\n{:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_double_simple() -> TestResult
    {
        let data = r#""hello world""#;
        let stats = &mut MStats::new();
        let expected = Token::Scalar(cow!("hello world"), ScalarStyle::DoubleQuote);

        let (range, read) = scan_flow_scalar(data, stats, false)?;
        let scalar = range.into_token(data)?;

        assert_eq!(read, 13);

        if !(scalar == expected)
        {
            bail!("expected\n{:?}\nbut got\n{:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_double_unicode_escape() -> TestResult
    {
        let data = r#""hello \U000003B1 \u03A9 \u30C3""#;
        let stats = &mut MStats::new();
        let expected = Token::Scalar(cow!("hello α Ω ッ"), ScalarStyle::DoubleQuote);

        let (range, read) = scan_flow_scalar(data, stats, false)?;
        let scalar = range.into_token(data)?;

        if !(scalar == expected)
        {
            bail!("expected\n{:?}\nbut got\n{:?}", expected, &scalar)
        }

        assert_eq!(
            read,
            data.len(),
            "expected to {} bytes, but got {}",
            data.len(),
            read
        );

        Ok(())
    }

    #[test]
    fn flow_double_fold_lines() -> TestResult
    {
        let data = r#""first
            second
            third
fourth""#;
        let stats = &mut MStats::new();
        let cmp = "first second third fourth";
        let expected = Token::Scalar(cow!(cmp), ScalarStyle::DoubleQuote);

        let (range, _read) = scan_flow_scalar(data, stats, false)?;
        let scalar = range.into_token(data)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_double_fold_newline() -> TestResult
    {
        let data = r#""first
            second
        third

            fourth""#;
        let stats = &mut MStats::new();
        let cmp = "first second third\nfourth";
        let expected = Token::Scalar(cow!(cmp), ScalarStyle::DoubleQuote);

        let (range, _read) = scan_flow_scalar(data, stats, false)?;
        let scalar = range.into_token(data)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_double_trim_whitespace() -> TestResult
    {
        let data = r#""first        
            second""#;
        let stats = &mut MStats::new();
        let cmp = "first second";
        let expected = Token::Scalar(cow!(cmp), ScalarStyle::DoubleQuote);

        let (range, _read) = scan_flow_scalar(data, stats, false)?;
        let scalar = range.into_token(data)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_double_escape_newline() -> TestResult
    {
        let data = r#""fi\
rst  \
            second
        third

            fourth""#;
        let stats = &mut MStats::new();
        let cmp = "first  second third\nfourth";
        let expected = Token::Scalar(cow!(cmp), ScalarStyle::DoubleQuote);

        let (range, _read) = scan_flow_scalar(data, stats, false)?;
        let scalar = range.into_token(data)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }
}
