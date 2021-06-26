//! This module exports function(s) for handling scalar
//! escapes in YAML documents.

use crate::scanner::error::{ScanError, ScanResult as Result};

/// Unescape a given YAML escape sequence as defined in
/// [Section 5.7][Link]. Specifically, YAML defines 18
/// 'special' escapes, and 3 methods of encoding 8, 16 and
/// 32 bit unicode points.
///
/// It writes the unescaped character to .scratch, returning
/// the length of .buffer advanced, or an error if the
/// escape sequence is invalid. It expects .buffer->0 is a
/// backslash (\\), as this is the only valid start of an
/// escape sequence.
///
/// [Link]: https://yaml.org/spec/1.2/spec.html#c-escape
pub(super) fn flow_unescape(base: &str, scratch: &mut Vec<u8>) -> Result<usize>
{
    let mut buffer = base;
    let mut escape_len: Option<u8> = None;

    // Not an escape sequence, early exit
    if !check!(~buffer => b'\\')
    {
        return Ok(0);
    }

    advance!(buffer, 1);

    // See 5.7: Escaped Characters
    // yaml.org/spec/1.2/spec.html#id2776092
    match buffer.as_bytes()
    {
        [b'0', ..] => scratch.push(b'\0'),
        [b'a', ..] => scratch.push(b'\x07'),
        [b'b', ..] => scratch.push(b'\x08'),
        [b't', ..] | [b'\t', ..] => scratch.push(b'\x09'),
        [b'n', ..] => scratch.push(b'\x0A'),
        [b'v', ..] => scratch.push(b'\x0B'),
        [b'f', ..] => scratch.push(b'\x0C'),
        [b'r', ..] => scratch.push(b'\x0D'),
        [b'e', ..] => scratch.push(b'\x1B'),
        [b' ', ..] => scratch.push(b'\x20'),
        [b'"', ..] => scratch.push(b'"'),
        // Forward slashes are not supported in the 1.1 spec
        [b'/', ..] => scratch.push(b'/'),
        [b'\\', ..] => scratch.push(b'\\'),
        [b'N', ..] => scratch.extend_from_slice(&NEL),
        [b'_', ..] => scratch.extend_from_slice(&NBS),
        [b'L', ..] => scratch.extend_from_slice(&LS),
        [b'P', ..] => scratch.extend_from_slice(&PS),
        [b'x', ..] => escape_len = Some(2),
        [b'u', ..] => escape_len = Some(4),
        [b'U', ..] => escape_len = Some(8),
        [] => return Err(ScanError::UnexpectedEOF),
        _ => return Err(ScanError::UnknownEscape),
    }
    advance!(buffer, 1);

    if let Some(sequence) = escape_len
    {
        let amt = write_unicode_point(buffer, scratch, sequence)?;
        advance!(buffer, amt);
    }

    Ok(base.len() - buffer.len())
}

/// Writes a UTF8 codepoint to the scratch space
fn write_unicode_point(base: &str, scratch: &mut Vec<u8>, codepoint_len: u8) -> Result<usize>
{
    let mut buffer = base;
    let mut i = 0;
    let mut value: u32 = 0;

    if codepoint_len < 1
    {
        return Ok(0);
    }

    while i < codepoint_len
    {
        match buffer.as_bytes().first()
        {
            None => return Err(ScanError::UnexpectedEOF),
            Some(c) if !c.is_ascii_hexdigit() => return Err(ScanError::UnknownEscape),

            Some(b) => value = (value << 4) + as_hex(*b) as u32,
        }
        advance!(buffer, 1, i);
    }

    // Bit shift the value into the correct byte configuration
    // for UTF8
    match value
    {
        // v <= 127 (ASCII)
        v if v <= 0x7F => scratch.push(v as u8),
        // v <= 2047
        v if v <= 0x7FF =>
        {
            scratch.extend_from_slice(&[0xC0 | (v >> 6) as u8, 0x80 | (v & 0x3F) as u8])
        },
        // v <= 65535
        v if v <= 0xFFFF => scratch.extend_from_slice(&[
            0xE0 | (v >> 12) as u8,
            0x80 | ((v >> 6) & 0x3F) as u8,
            0x80 | (v & 0x3F) as u8,
        ]),
        // Otherwise it must be a full 4 byte code point
        v => scratch.extend_from_slice(&[
            0xF0 | (v >> 18) as u8,
            0x80 | ((v >> 12) & 0x3F) as u8,
            0x80 | ((v >> 6) & 0x3F) as u8,
            0x80 | (v & 0x3F) as u8,
        ]),
    }

    Ok(codepoint_len as usize)
}

/*
 * Inclusive range suggested by clippy here is 5-10%
 * slower than doing it by hand, see
 *
 * github.com/rust-lang/rust/issues/45222
 */
#[allow(clippy::manual_range_contains)]
#[inline]
fn as_hex(b: u8) -> u8
{
    let ret = if b >= b'A' && b <= b'F'
    {
        b - b'A' + 10
    }
    else if b >= b'a' && b <= b'f'
    {
        b - b'a' + 10
    }
    else
    {
        b - b'0'
    };

    ret
}

/// <Next Line> (U+0085)
const NEL: [u8; 2] = [b'\xC2', b'\x85'];
/// <No-Break Space> (U+00A0)
const NBS: [u8; 2] = [b'\xC2', b'\xA0'];
/// <Line Separator> (U+2028)
const LS: [u8; 3] = [b'\xE2', b'\x80', b'\xA8'];
/// <Paragraph Separator> (U+2029)
const PS: [u8; 3] = [b'\xE2', b'\x80', b'\xA9'];

#[cfg(test)]
mod tests
{
    use anyhow::anyhow;
    use pretty_assertions::assert_eq;

    use super::*;

    type TestResult = anyhow::Result<()>;

    #[test]
    fn flow_escape_special() -> TestResult
    {
        let mut s = Vec::new();
        let scratch = &mut s;
        let data = &[
            //  0       1        2        3        4        5        6         7        8
            r#"\0"#, r#"\a"#, r#"\b"#, r#"\t"#, r#"\	"#, r#"\n"#, r#"\v"#, r#"\f"#, r#"\r"#,
            //  9      10       11       12       13       14       15       16       17
            r#"\e"#, r#"\ "#, r#"\""#, r#"\/"#, r#"\\"#, r#"\N"#, r#"\_"#, r#"\L"#, r#"\P"#,
        ];
        let expected: &[&[u8]] = &[
            &[b'\0'],   // 0
            &[b'\x07'], // 1
            &[b'\x08'], // 2
            &[b'\x09'], // 3
            &[b'\x09'], // 4
            &[b'\x0A'], // 5
            &[b'\x0B'], // 6
            &[b'\x0C'], // 7
            &[b'\x0D'], // 8
            &[b'\x1B'], // 9
            &[b'\x20'], // 10
            &[b'"'],    // 11
            &[b'/'],    // 12
            &[b'\\'],   // 13
            &NEL,       // 14
            &NBS,       // 15
            &LS,        // 16
            &PS,        // 17
        ];

        assert_eq!(
            data.len(),
            expected.len(),
            "test data length != expected length"
        );

        for (i, (&t, &ex)) in data.into_iter().zip(expected).enumerate()
        {
            scratch.clear();
            flow_unescape(t, scratch)
                .map_err(|e| anyhow!("on iteration {}, test errored with {}", i, e))?;

            assert_eq!(scratch, ex, "on iteration {}", i)
        }

        Ok(())
    }

    #[test]
    fn flow_escape_hex() -> TestResult
    {
        let mut s = Vec::new();
        let scratch = &mut s;
        #[rustfmt::skip]
        let data = &[
                                // === 1 byte
            r#"\x64"#,          // 0
            r#"\x65"#,          // 1
                                // === 2 bytes
            r#"\x7f"#,          // 2
            r#"\xF7"#,          // 3
            r#"\xB6"#,          // 4
            r#"\xFF"#,          // 5
            r#"\xC6"#,          // 6
            r#"\u2c61"#,        // 7
            r#"\u30C4"#,        // 8
            r#"\ua026"#,        // 9
                                // === 4 bytes
            r#"\U000111E1"#,    // 10
        ];
        #[rustfmt::skip]
        let expected = &[
                                // === 1 byte
            'd',                // 0
            'e',                // 1
                                // === 2 bytes
            '\u{7f}',           // 2
            '÷',                // 3
            '¶',                // 4
            'ÿ',                // 5
            'Æ',                // 6
                                // === 3 bytes
            'ⱡ',                // 7
            'ツ',               // 8
            'ꀦ',               // 9
                                // === 4 bytes
            '𑇡'                 // 10
        ];

        assert_eq!(
            data.len(),
            expected.len(),
            "test data length != expected length"
        );

        for (i, (&t, &ex)) in data.into_iter().zip(expected).enumerate()
        {
            let mut c: [u8; 4] = [0; 4];
            scratch.clear();

            flow_unescape(t, scratch)
                .map_err(|e| anyhow!("on iteration {}, test errored with {}", i, e))?;

            assert_eq!(
                scratch,
                ex.encode_utf8(&mut c).as_bytes(),
                "on iteration {}, codepoint '{}'",
                i,
                ex
            )
        }

        Ok(())
    }

    #[test]
    fn flow_escape_consumed() -> TestResult
    {
        let mut s = Vec::new();
        let scratch = &mut s;
        let data = &[
            // === 1 byte
            r#"\x64"#, // 0
            r#"\x65"#, // 1
            // === 2 bytes
            r#"\x7f"#,   // 2
            r#"\xF7"#,   // 3
            r#"\xB6"#,   // 4
            r#"\xFF"#,   // 5
            r#"\xC6"#,   // 6
            r#"\u2c61"#, // 7
            r#"\u30C4"#, // 8
            r#"\ua026"#, // 9
            // === 4 bytes
            r#"\U000111E1"#, // 10
        ];

        for (i, &t) in data.into_iter().enumerate()
        {
            scratch.clear();

            let consumed = flow_unescape(t, scratch)
                .map_err(|e| anyhow!("on iteration {}, test errored with {}", i, e))?;

            assert_eq!(
                consumed,
                t.len(),
                "on iteration {}, expected to consume {}, got {}",
                i,
                t.len(),
                consumed
            )
        }

        Ok(())
    }
}
