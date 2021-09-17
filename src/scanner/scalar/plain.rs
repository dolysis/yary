use crate::{
    scanner::{
        context::Context,
        entry::MaybeToken,
        error::{ScanError, ScanResult as Result},
        flag::{Flags, O_EXTENDABLE, O_LAZY},
        scalar::as_maybe,
        stats::MStats,
    },
    token::{ScalarStyle, Token},
};

pub(in crate::scanner) fn scan_plain_scalar<'de>(
    opts: Flags,
    base: &'de str,
    stats: &mut MStats,
    cxt: &Context,
) -> Result<(MaybeToken<'de>, usize)>
{
    // Ensure the scalar is at least one column above the most
    // recent indentation level
    let indent = cxt.indent() + 1;
    let block_context = cxt.is_block();

    match opts.contains(O_LAZY)
    {
        true => scan_plain_scalar_lazy(opts, base, stats, indent, block_context).map(as_maybe),
        false => scan_plain_scalar_eager(opts, base, stats, indent, block_context).map(as_maybe),
    }
}

/// Scans a plain scalar, returning a Token, and the amount
/// read from .base. This function will attempt to borrow
/// from .base, however it may be required to copy into a
/// new allocation if line joining is required in the
/// scalar.
///
/// See:
///     YAML 1.2: Section 7.3.3
///     yaml.org/spec/1.2/spec.html#ns-plain-first(c)
pub(in crate::scanner) fn scan_plain_scalar_eager<'de>(
    opts: Flags,
    base: &'de str,
    stats: &mut MStats,
    indent: usize,
    block_context: bool,
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
    let flow_context = !block_context;

    // Inside flow contexts you *may not* start a plain scalar
    // with a ':', '?', or '-' followed by a flow indicator
    cache!(~buffer, 2, opts)?;
    if flow_context && check!(~buffer => b':' | b'?' | b'-') && flow_indicator(buffer, 1)
    {
        return Err(ScanError::InvalidPlainScalar);
    }

    'scalar: loop
    {
        // 4 is the largest character sequence we can encounter
        // (document indicators)
        cache!(~buffer, 4, opts)?;

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

        // Check for character sequences which end a plain scalar,
        // namely:
        //
        // ': '                         -> anywhere
        // ',' | '[' | ']' | '{' | '}'  -> flow context
        if (check!(~buffer => b':') && isWhiteSpaceZ!(~buffer, 1))
            || flow_context && flow_indicator(buffer, 0)
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
            cache!(~buffer, 2, opts)?;

            if (check!(~buffer => b':') && isWhiteSpaceZ!(~buffer, 1))
                || flow_context && flow_indicator(buffer, 0)
            {
                break;
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
            cache!(~buffer, 1, opts)?;

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
        outdent = block_context && local_stats.column < indent;

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
                // post loop truncate doesn't remove characters we've
                // already removed here
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
                lines -= 1;

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

pub(in crate::scanner) fn scan_plain_scalar_lazy<'de>(
    opts: Flags,
    base: &'de str,
    stats: &mut MStats,
    indent: usize,
    block_context: bool,
) -> Result<(Deferred<'de>, usize)>
{
    let mut buffer = base;

    let mut local_stats = stats.clone();
    let mut scalar_stats = stats.clone();

    // Have we hit a lower indentation to our starting indent?
    let mut outdent = false;

    // Are we in block/flow context?
    let flow_context = !block_context;

    'scalar: loop
    {
        // 4 is the largest character sequence we can encounter
        // (document indicators)
        cache!(~buffer, 4, opts)?;

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

        // Check for character sequences which end a plain scalar,
        // namely:
        //
        // ': '                         -> anywhere
        // ',' | '[' | ']' | '{' | '}'  -> flow context
        if (check!(~buffer => b':') && isWhiteSpaceZ!(~buffer, 1))
            || flow_context && flow_indicator(buffer, 0)
        {
            break 'scalar;
        }

        // Handle non whitespace characters
        while !isWhiteSpaceZ!(~buffer)
        {
            cache!(~buffer, 2, opts)?;

            if (check!(~buffer => b':') && isWhiteSpaceZ!(~buffer, 1))
                || flow_context && flow_indicator(buffer, 0)
            {
                break;
            }

            advance!(buffer, :local_stats, 1);
        }
        // Save last non whitespace character position
        scalar_stats = local_stats.clone();

        // Handle whitespace characters
        loop
        {
            cache!(~buffer, 1, opts)?;

            match (isBlank!(~buffer), isBreak!(~buffer))
            {
                // No more whitespace, exit loop
                (false, false) => break,
                // Handle non break space
                (true, _) =>
                {
                    advance!(buffer, :local_stats, 1);
                },
                // Handle line breaks
                (false, _) =>
                {
                    advance!(buffer, :local_stats, @line);
                },
            }
        }

        // If the whitespace ended at a lower indent, then we're
        // done, and should exit on the next loop
        outdent = block_context && local_stats.column < indent;
    }

    let advance = scalar_stats.read - stats.read;
    let slice = &base[..advance];

    // Note we remove O_EXTENDABLE as we've already located the
    // entire scalar
    let lazy = Deferred::new(
        opts & !O_EXTENDABLE,
        slice,
        stats.clone(),
        indent,
        block_context,
    );

    *stats = scalar_stats;

    Ok((lazy, advance))
}

/// Have we hit a flow context scalar end indicator?
fn flow_indicator(buffer: &str, offset: usize) -> bool
{
    check!(~buffer, offset => b',' | b'[' | b']' | b'{' | b'}')
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

#[derive(Debug, Clone)]
pub(in crate::scanner) struct Deferred<'de>
{
    opts:          Flags,
    slice:         &'de str,
    stats:         MStats,
    indent:        usize,
    block_context: bool,
}

impl<'de> Deferred<'de>
{
    pub fn new(
        opts: Flags,
        slice: &'de str,
        stats: MStats,
        indent: usize,
        block_context: bool,
    ) -> Self
    {
        Self {
            opts,
            slice,
            stats,
            indent,
            block_context,
        }
    }

    pub fn into_token(self) -> Result<Token<'de>>
    {
        let Deferred {
            opts,
            slice,
            mut stats,
            indent,
            block_context,
        } = self;

        scan_plain_scalar_eager(opts, slice, &mut stats, indent, block_context).map(|(t, _)| t)
    }
}

const SPACE: u8 = b' ';
const NEWLINE: u8 = b'\n';

#[cfg(test)]
mod tests
{
    use anyhow::anyhow;
    use pretty_assertions::assert_eq;
    use ScalarStyle::Plain;

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

    #[test]
    fn end_on_doc() -> TestResult
    {
        let tests = ["hello\n---\n", "hello\n... "];
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("hello"), Plain);

        for (i, &data) in tests.iter().enumerate()
        {
            let (token, amt) = scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt)
                .and_then(normalize)
                .map_err(|e| anyhow!("iteration {}: {}", i, e))?;

            assert_eq!(token, expected, "on iteration {}", i);

            assert_eq!(amt, 5, "on iteration {}", i);
        }

        Ok(())
    }

    #[test]
    fn end_on_comment() -> TestResult
    {
        let tests = ["hello #", "hello\n#"];
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("hello"), Plain);

        for (i, &data) in tests.iter().enumerate()
        {
            let (token, amt) = scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt)
                .and_then(normalize)
                .map_err(|e| anyhow!("iteration {}: {}", i, e))?;

            assert_eq!(token, expected, "on iteration {}", i);

            assert_eq!(amt, 5, "on iteration {}", i);
        }

        Ok(())
    }

    #[test]
    fn empty() -> TestResult
    {
        let data = "# a comment";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!(""), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, 0);

        Ok(())
    }

    /* === BLOCK CONTEXT === */

    #[test]
    fn block_simple() -> TestResult
    {
        let data = "hello";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("hello"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, data.len());

        Ok(())
    }

    #[test]
    fn block_simple_key() -> TestResult
    {
        let data = "hello, world!: ";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("hello, world!"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, 13);

        Ok(())
    }

    #[test]
    fn block_multi_line() -> TestResult
    {
        let data = "hello
 this
 is
 a
 multi-line
 scalar
";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("hello this is a multi-line scalar"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, data.trim_end().len());

        Ok(())
    }

    #[test]
    fn block_multi_line_breaks() -> TestResult
    {
        let data = "this
 is


 a
 scalar

 with
 line#breaks

";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("this is\n\na scalar\nwith line#breaks"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, data.trim_end().len());

        Ok(())
    }

    #[test]
    fn block_trailing_whitespace() -> TestResult
    {
        let data = "hello       ";
        let mut stats = MStats::new();
        let cxt = cxt!(block -> [0]);
        let expected = Token::Scalar(cow!("hello"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, 5);

        Ok(())
    }

    /* === FLOW CONTEXT === */

    #[test]
    fn flow_simple() -> TestResult
    {
        let data = "hello";
        let mut stats = MStats::new();
        let cxt = cxt!(flow -> 1);
        let expected = Token::Scalar(cow!("hello"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, data.len());

        Ok(())
    }

    #[test]
    fn flow_end_on_indicator() -> TestResult
    {
        let tests = ["hello: ", "hello,", "hello[", "hello]", "hello{", "hello}"];
        let mut stats = MStats::new();
        let cxt = cxt!(flow -> 1);
        let expected = Token::Scalar(cow!("hello"), Plain);

        for (i, &data) in tests.iter().enumerate()
        {
            let (token, amt) = scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt)
                .and_then(normalize)
                .map_err(|e| anyhow!("iteration {}: {}", i, e))?;

            assert_eq!(token, expected, "on iteration {}", i);

            assert_eq!(amt, 5, "on iteration {}", i);
        }

        Ok(())
    }

    #[test]
    fn flow_multi_line() -> TestResult
    {
        let data = "hello
this
is
a
multi-line
string!";
        let mut stats = MStats::new();
        let cxt = cxt!(flow -> 1);
        let expected = Token::Scalar(cow!("hello this is a multi-line string!"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, data.len());

        Ok(())
    }

    #[test]
    fn flow_multi_line_breaks() -> TestResult
    {
        let data = "hello
        this

big

    string

        has

    line

breaks
        ";
        let mut stats = MStats::new();
        let cxt = cxt!(flow -> 1);
        let expected = Token::Scalar(cow!("hello this\nbig\nstring\nhas\nline\nbreaks"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, 66);

        Ok(())
    }

    #[test]
    fn flow_trailing_whitespace_key() -> TestResult
    {
        let data = "hello   : ";
        let mut stats = MStats::new();
        let cxt = cxt!(flow -> 1);
        let expected = Token::Scalar(cow!("hello"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, 5);

        Ok(())
    }

    #[test]
    fn flow_trailing_whitespace() -> TestResult
    {
        let data = "hello   ";
        let mut stats = MStats::new();
        let cxt = cxt!(flow -> 1);
        let expected = Token::Scalar(cow!("hello"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, 5);

        Ok(())
    }

    #[test]
    fn flow_trailing_breaks() -> TestResult
    {
        let data = "hello

        
        
        ";
        let mut stats = MStats::new();
        let cxt = cxt!(flow -> 1);
        let expected = Token::Scalar(cow!("hello"), Plain);

        let (token, amt) =
            scan_plain_scalar(TEST_FLAGS, data, &mut stats, &cxt).and_then(normalize)?;

        assert_eq!(token, expected);

        assert_eq!(amt, 5);

        Ok(())
    }
}
