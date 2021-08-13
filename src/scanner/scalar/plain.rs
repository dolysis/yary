use crate::{
    scanner::{
        context::Context,
        error::{ScanError, ScanResult as Result},
        MStats,
    },
    token::{ScalarStyle, Token},
};

/// Scans a plain scalar, returning a Token, and the amount
/// read from .base. This function will attempt to borrow
/// from .base, however it may be required to copy into a
/// new allocation if line joining is required in the
/// scalar.
///
/// See:
///     YAML 1.2: Section 7.3.3
///     yaml.org/spec/1.2/spec.html#ns-plain-first(c)
pub(in crate::scanner) fn scan_plain_scalar<'de>(
    base: &'de str,
    stats: &mut MStats,
    cxt: &Context,
) -> Result<(Token<'de>, usize)>
{
    let mut buffer = base;
    let mut scratch = Vec::new();

    // Local copies of the given stats
    let mut local_stats = stats.clone();
    let mut scalar_stats = stats.clone();

    // Do we need to normalize and therefore allocate?
    let mut can_borrow = true;
    // Have we hit a lower indentation to our starting indent?
    let mut outdent = false;

    // Track whitespace, line breaks accumulated, these have two
    // uses:
    //
    // 1. In loop, for handling line joins
    // 2. Post loop for truncating trailing space
    let mut whitespace: usize = 0;
    let mut lines: usize = 0;

    // Are we in block/flow context?
    let block_context = cxt.is_block();
    let flow_context = !block_context;

    // Have we hit a flow context scalar end indicator?
    let flow_indicator =
        |buffer: &str, at: usize| check!(~buffer, at => b',' | b'[' | b']' | b'{' | b'}');

    // Inside flow contexts you *may not* start a plain scalar
    // with a ':', '?', or '-' followed by a flow indicator
    if flow_context && check!(~buffer => b':' | b'?' | b'-') && flow_indicator(buffer, 1)
    {
        return Err(ScanError::InvalidPlainScalar);
    }

    'scalar: loop
    {
        if buffer.is_empty()
        {
            break 'scalar;
        }

        if outdent
        {
            break 'scalar;
        }

        // A YAML document indicator or ' #' terminates a plain
        // scalar
        //
        // Note that due to how this function is setup, the _only_
        // times we will hit this guard is if:
        //
        // 1. We've just started the function, and thus we were
        // called on a non whitespace character
        //
        // 2. We've gone through the loop, exhausting any
        // whitespace, thus hitting this guard again
        //
        // Therefore just checking for '#' is okay
        if isDocumentIndicator!(~buffer, :local_stats) || check!(~buffer => b'#')
        {
            break 'scalar;
        }

        // Reset whitespace counters for next char / whitespace
        // sequence. We do this here after all possible terminations
        // that could leave trailing whitespace, so we can
        // accurately truncate the trailing whitespace post
        // loop.
        whitespace = 0;
        lines = 0;

        // Handle non whitespace characters
        while !isWhiteSpaceZ!(~buffer)
        {
            // Check for character sequences which end a plain scalar,
            // namely:
            //
            // ': '                         -> anywhere
            // ',' | '[' | ']' | '{' | '}'  -> flow context
            if (check!(~buffer => b':') && isWhiteSpaceZ!(~buffer, 1))
                || flow_context && flow_indicator(buffer, 0)
            {
                // Save the position of the last known non whitespace
                // character, so we can truncate when trimming trailing
                // whitespace
                scalar_stats = local_stats.clone();

                break 'scalar;
            }

            if !can_borrow
            {
                scratch.push(buffer.as_bytes()[0])
            }
            advance!(buffer, :local_stats, 1);
        }
        // Save last non whitespace character position
        scalar_stats = local_stats.clone();

        // Handle whitespace characters
        loop
        {
            match (isBlank!(~buffer), isBreak!(~buffer))
            {
                // No more whitespace, exit loop
                (false, false) => break,
                // Handle non break space
                (true, _) =>
                {
                    if !can_borrow
                    {
                        scratch.push(buffer.as_bytes()[0])
                    }
                    whitespace += 1;
                    advance!(buffer, :local_stats, 1);
                },
                // Handle line breaks
                (false, _) =>
                {
                    set_no_borrow(&mut can_borrow, base, buffer, &mut scratch);

                    lines += 1;
                    advance!(buffer, :local_stats, @line);
                },
            }
        }

        // If the whitespace ended at a lower indent, then we're
        // done, and should exit on the next loop
        outdent = block_context && cxt.indent() > stats.column;

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
                // Note that we reset whitespace to zero here, so that the
                // post loop truncate doesn't
                // remove characters we've already removed here
                scratch.truncate(scratch.len() - whitespace);
                whitespace = 0;

                scratch.push(SPACE);
            },
            // Else we need to append (n - 1) newlines, as we skip the origin line's break
            _ =>
            {
                // Similarly, we reset whitespace here, but we _also_ set
                // lines to the amount of lines we actually add to the
                // scratch space.
                scratch.truncate(scratch.len() - whitespace);
                whitespace = 0;
                lines = lines - 1;

                // Safety: we can only reach this branch if lines > 1
                for _ in 0..lines
                {
                    scratch.push(NEWLINE)
                }
            },
        }
    }

    // Trim any trailing whitespace that might be left after
    // exiting the loop
    if !can_borrow
    {
        scratch.truncate(scratch.len() - (whitespace + lines));
    }
    // Note we use the stats which point at the last word read
    let advance = scalar_stats.read - stats.read;

    let slice = match can_borrow
    {
        true => cow!(&base[..advance]),
        false =>
        {
            let utf8 = String::from_utf8(scratch).unwrap();

            cow!(utf8)
        },
    };

    let token = Token::Scalar(slice, ScalarStyle::Plain);
    *stats = scalar_stats;

    Ok((token, advance))
}

/// Handles the trap door from borrowing to copying
fn set_no_borrow(can_borrow: &mut bool, base: &str, buffer: &str, scratch: &mut Vec<u8>)
{
    if *can_borrow
    {
        scratch.extend_from_slice(base[0..base.len() - buffer.len()].as_bytes());
    }

    *can_borrow = false
}

const SPACE: u8 = b' ';
const NEWLINE: u8 = b'\n';
