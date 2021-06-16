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
    let mut start: u64;

    // Eat left quote
    advance!(buffer, 1);

    start = 1;

    'scalar: loop
    {
        // It is not legal to start a flow scalar with document
        // start/end markers
        if buffer.starts_with("---") || buffer.starts_with("...")
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
            // from the backing slice, we must unescape the quote
            if check!(~buffer => [SINGLE, SINGLE, ..])
            {
                can_borrow = false;

                scratch.push(SINGLE);
                advance!(buffer, 2, start);
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
            advance!(buffer, 1, start);
        }

        // Consume whitespace
        while isBlank!(~buffer) || isBreak!(~buffer)
        {
            if isBlank!(~buffer)
            {
                if !can_borrow
                {
                    scratch.push(buffer.as_bytes()[0])
                }
                advance!(buffer, 1, start);
            }
            else
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
                // line consists solely of a break we add it
                // literally, otherwise we eat blanks
                // until we find a char
            }
        }
    }

    // Retrieve the token slice, either from the base slice, or
    // if we couldn't borrow, the scratch space
    let token = if can_borrow
    {
        let s = (&*base).get(1..buffer.len()).unwrap();
        let token = Token::Scalar(cow!(s), ScalarStyle::SingleQuote);

        Ref::Borrow(token)
    }
    else
    {
        let s = std::str::from_utf8(scratch).unwrap();
        let token = Token::Scalar(cow!(s), ScalarStyle::SingleQuote);

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
pub enum Ref<'borrow, 'copy>
{
    Borrow(Token<'borrow>),
    Copy(Token<'copy>),
}
