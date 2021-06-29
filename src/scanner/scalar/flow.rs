use crate::{
    scanner::{
        error::{ScanError, ScanResult as Result},
        scalar::escape::flow_unescape,
        MStats,
    },
    token::{Ref, ScalarStyle, Token},
};

/// Scans a single or double quoted (flow) scalar returning
/// a Token containing the contents, and the amount read
/// from .base. This function will attempt to borrow from
/// the underlying .base, however it may be required to copy
/// into .scratch and borrow from that lifetime.
pub(in crate::scanner) fn scan_flow_scalar<'b, 'c>(
    base: &'b str,
    stats: &mut MStats,
    scratch: &'c mut Vec<u8>,
    single: bool,
) -> Result<(Ref<'b, 'c>, usize)>
{
    use ScalarStyle::{DoubleQuote, SingleQuote};

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
        if stats.column == 0
            && check!(~buffer => [b'-', b'-', b'-', ..] | [b'.', b'.', b'.', ..])
            && isWhiteSpaceZ!(~buffer, 3)
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
                set_no_borrow(&mut can_borrow, base, buffer, scratch);

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
                set_no_borrow(&mut can_borrow, base, buffer, scratch);

                escaped_break = Some(EscapeState::Start);
                advance!(buffer, :stats, 1);
            }
            // We've hit an escape sequence, parse it
            else if kind == DoubleQuote && check!(~buffer => [BACKSLASH, ..])
            {
                set_no_borrow(&mut can_borrow, base, buffer, scratch);

                let read = flow_unescape(buffer, scratch)?;
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
                        whitespace += 1;
                        scratch.push(buffer.as_bytes()[0]);
                    }
                    advance!(buffer, :stats, 1);
                },
                // Handle line breaks
                (false, _) =>
                {
                    set_no_borrow(&mut can_borrow, base, buffer, scratch);

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
                set_no_borrow(&mut can_borrow, base, buffer, scratch);

                scratch.truncate(scratch.len() - whitespace);

                if escaped_break.is_none()
                {
                    scratch.push(SPACE);
                }
            },
            // Else we need to append (n - 1) newlines, as we skip the origin line's break
            n =>
            {
                set_no_borrow(&mut can_borrow, base, buffer, scratch);

                scratch.truncate(scratch.len() - whitespace);

                // Safety: we can only reach this branch if n > 1
                for _ in 0..n - 1
                {
                    scratch.push(NEWLINE)
                }
            },
        }
    }

    // Retrieve the token slice, either from the .base slice, or
    // if we couldn't borrow, the .scratch space
    let token = if can_borrow
    {
        // Safety: we must be on a code point boundary, as the only
        // way can get to this section is:
        //
        // 1. .base->0 must be a quote
        // 2. .base->.base.len() - .buffer.len() must be a quote
        // 3. .base must be valid UTF8 (its a str)
        let fragment = &base[1..base.len() - buffer.len()];
        let token = Token::Scalar(cow!(fragment), kind);

        Ref::Borrow(token)
    }
    else
    {
        // Safety: characters added to scratch are either:
        //
        // A. added from a str (.base)
        // B. Unescaped into valid UTF8
        let fragment = std::str::from_utf8(scratch).unwrap();
        let token = Token::Scalar(cow!(fragment), kind);

        Ref::Copy(token)
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
        let scratch = &mut Vec::new();
        let expected = Ref::Borrow(Token::Scalar(cow!(""), ScalarStyle::SingleQuote));

        let (scalar, read) = scan_flow_scalar(data, scratch, true)?;

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
        let scratch = &mut Vec::new();
        let expected = Ref::Borrow(Token::Scalar(cow!("hello world"), ScalarStyle::SingleQuote));

        let (scalar, read) = scan_flow_scalar(data, scratch, true)?;

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
        let scratch = &mut Vec::new();
        let cmp = "first second third fourth";
        let expected = Ref::Copy(Token::Scalar(cow!(cmp), ScalarStyle::SingleQuote));

        let (scalar, _read) = scan_flow_scalar(data, scratch, true)?;

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
        let scratch = &mut Vec::new();
        let cmp = "first second third\nfourth";
        let expected = Ref::Copy(Token::Scalar(cow!(cmp), ScalarStyle::SingleQuote));

        let (scalar, _read) = scan_flow_scalar(data, scratch, true)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }

    #[test]
    fn flow_single_reject_document()
    {
        let data = ["'--- '", "'---\n'"];
        let scratch = &mut Vec::new();
        let expected = ScanError::InvalidFlowScalar;

        for (i, &t) in (&data).into_iter().enumerate()
        {
            match scan_flow_scalar(t, scratch, true)
            {
                Err(e) => assert_eq!(
                    e, expected,
                    "on iteration {}, expected error {}, got {}",
                    i, expected, e
                ),
                Ok((unexpected, _)) => panic!(
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
        let scratch = &mut Vec::new();
        let expected = ScanError::UnexpectedEOF;

        for (i, &t) in (&data).into_iter().enumerate()
        {
            match scan_flow_scalar(t, scratch, true)
            {
                Err(e) => assert_eq!(
                    e, expected,
                    "on iteration {}, expected error {}, got {}",
                    i, expected, e
                ),
                Ok((unexpected, _)) => panic!(
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
        let scratch = &mut Vec::new();
        let expected = Ref::Borrow(Token::Scalar(cow!(""), ScalarStyle::DoubleQuote));

        let (scalar, read) = scan_flow_scalar(data, scratch, false)?;

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
        let scratch = &mut Vec::new();
        let expected = Ref::Borrow(Token::Scalar(cow!("hello world"), ScalarStyle::DoubleQuote));

        let (scalar, read) = scan_flow_scalar(data, scratch, false)?;

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
        let scratch = &mut Vec::new();
        let expected = Ref::Copy(Token::Scalar(
            cow!("hello α Ω ッ"),
            ScalarStyle::DoubleQuote,
        ));

        let (scalar, read) = scan_flow_scalar(data, scratch, false)?;

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
        let scratch = &mut Vec::new();
        let cmp = "first second third fourth";
        let expected = Ref::Copy(Token::Scalar(cow!(cmp), ScalarStyle::DoubleQuote));

        let (scalar, _read) = scan_flow_scalar(data, scratch, false)?;

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
        let scratch = &mut Vec::new();
        let cmp = "first second third\nfourth";
        let expected = Ref::Copy(Token::Scalar(cow!(cmp), ScalarStyle::DoubleQuote));

        let (scalar, _read) = scan_flow_scalar(data, scratch, false)?;

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
        let scratch = &mut Vec::new();
        let cmp = "first  second third\nfourth";
        let expected = Ref::Copy(Token::Scalar(cow!(cmp), ScalarStyle::DoubleQuote));

        let (scalar, _read) = scan_flow_scalar(data, scratch, false)?;

        if !(scalar == expected)
        {
            bail!("\nexpected: {:?}\nbut got: {:?}", expected, &scalar)
        }

        Ok(())
    }
}
