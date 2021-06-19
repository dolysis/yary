use crate::{
    scanner::error::{ScanError, ScanResult as Result},
    token::{ScalarStyle, Token},
};

const SINGLE: u8 = b'\'';

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
                can_borrow = false;

                scratch.push(SINGLE);
                advance!(buffer, 2);
            }

            // We're done, we hit the right quote
            if check!(~buffer => [SINGLE, ..])
            {
                break 'scalar;
            }

            // Its a non blank character, add it
            if !can_borrow
            {
                // Safety: isBlankZ guarantees the slice is not empty
                scratch.push(buffer.as_bytes()[0])
            }
            advance!(buffer, 1);
        }

        // Consume whitespace
        loop
        {
            match (isBlank!(~buffer), isBreak!(~buffer))
            {
                (false, false) => break,
                (true, _) =>
                {
                    if !can_borrow
                    {
                        scratch.push(buffer.as_bytes()[0])
                    }
                    advance!(buffer, 1);
                },
                (false, _) =>
                {
                    // need to handle potential joins
                    // e.g ===================
                    //  'a                  'a
                    //   b                   b
                    //   c
                    //   d'                  c'
                    //   -> 'a b c d'        -> 'a b \nc'
                    //
                    // Seems like the rule here is that if
                    // line consists solely of a break we
                    // add it literally,
                    // otherwise we eat blanks
                    // until we find a char
                    unimplemented!(
                        "handling of line breaks in flow scalars is not implemented yet!"
                    )
                },
            }
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
        // 2. .base->.buffer.len() - 1 must be a quote
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
