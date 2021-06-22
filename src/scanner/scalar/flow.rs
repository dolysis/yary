use crate::{
    scanner::error::{ScanError, ScanResult as Result},
    token::{ScalarStyle, Token},
};

fn scan_flow_scalar_single_quote<'b, 'c>(
    base: &'b str,
    scratch: &'c mut Vec<u8>,
) -> Result<(Ref<'b, 'c>, usize)>
{
    let mut buffer = base;
    let mut can_borrow = true;

    // Eat left quote
    advance!(buffer, 1);

    'scalar: loop
    {
        // Even in a scalar context, YAML prohibits starting a line
        // with document stream tokens followed by a blank
        // character
        //
        // FIXME: this is currently incorrect, we also need to check
        // that we are at the beginning of a line which would
        // require us tracking columns.
        //
        // This should be fixed once I figure out how to do that
        if check!(~buffer => [b'-', b'-', b'-', ..] | [b'.', b'.', b'.', ..])
            && isBlankZ!(~buffer, 3)
        {
            return Err(ScanError::InvalidFlowScalar);
        }

        // EOF without a ' is an error
        if buffer.is_empty()
        {
            return Err(ScanError::UnexpectedEOF);
        }

        // Consume non whitespace characters
        while !isBlankZ!(~buffer)
        {
            // if we encounter an escaped quote we can no longer borrow
            // from .base, we must unescape the quote into .scratch
            if check!(~buffer => [SINGLE, SINGLE, ..])
            {
                set_no_borrow(&mut can_borrow, base, buffer, scratch);

                scratch.push(SINGLE);
                advance!(buffer, 2);
            }
            // We're done, we hit the right quote
            else if check!(~buffer => [SINGLE, ..])
            {
                break 'scalar;
            }
            // Its a non blank character, add it
            else
            {
                if !can_borrow
                {
                    // Safety: !isBlankZ guarantees the slice is not empty
                    scratch.push(buffer.as_bytes()[0])
                }
                advance!(buffer, 1);
            }
        }

        // let mut join = None;
        let mut whitespace: usize = 0;
        let mut lines: usize = 0;

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
                    advance!(buffer, 1);
                },
                // Handle line breaks
                (false, _) =>
                {
                    set_no_borrow(&mut can_borrow, base, buffer, scratch);

                    lines += 1;
                    advance!(buffer, 1);
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

                scratch.push(SPACE);
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
        // 2. .base->.base.len() - buffer.len() must be a quote
        // 3. .base must be valid UTF8 (its a str)
        let fragment = base.get(1..base.len() - buffer.len()).unwrap();
        let token = Token::Scalar(cow!(fragment), ScalarStyle::SingleQuote);

        Ref::Borrow(token)
    }
    else
    {
        // Safety: characters added to scratch are either:
        //
        // A. added from a str (.base)
        // B. Unescaped into valid UTF8
        let fragment = std::str::from_utf8(scratch).unwrap();
        let token = Token::Scalar(cow!(fragment), ScalarStyle::SingleQuote);

        Ref::Copy(token)
    };

    // Eat the right quote
    advance!(buffer, 1);

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

/// This allows us to discriminate between a Token with
/// different lifetimes, specifically either a lifetime
/// 'borrow-ed from the underlying data or 'copy-ied from
/// some scratch space provided.
#[derive(Debug, PartialEq)]
pub enum Ref<'borrow, 'copy>
{
    Borrow(Token<'borrow>),
    Copy(Token<'copy>),
}

const SINGLE: u8 = b'\'';
const SPACE: u8 = b' ';
const NEWLINE: u8 = b'\n';

#[cfg(test)]
mod tests
{
    use anyhow::bail;
    use pretty_assertions::assert_eq;

    use super::*;

    type TestResult = anyhow::Result<()>;

    #[test]
    fn flow_single_empty() -> TestResult
    {
        let data = "''";
        let scratch = &mut Vec::new();
        let expected = Ref::Borrow(Token::Scalar(cow!(""), ScalarStyle::SingleQuote));

        let (scalar, read) = scan_flow_scalar_single_quote(data, scratch)?;

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

        let (scalar, read) = scan_flow_scalar_single_quote(data, scratch)?;

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

        let (scalar, _read) = scan_flow_scalar_single_quote(data, scratch)?;

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

        let (scalar, _read) = scan_flow_scalar_single_quote(data, scratch)?;

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
            match scan_flow_scalar_single_quote(t, scratch)
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
            match scan_flow_scalar_single_quote(t, scratch)
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
}
