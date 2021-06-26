//! This module exports function(s) for handling scalar
//! escapes in YAML documents.

use crate::scanner::error::{ScanError, ScanResult as Result};

/// Unescape a given YAML escape sequence as defined in
/// [Section 5.7][Link]. Specifically, YAML defines 18
/// 'special' escapes, and 3 methods of encoding 8, 16 and
/// 32 bit unicode points.
///
/// It writes the unescaped character to .scratch, returning
/// the length of .base advanced, or an error if the
/// escape sequence is invalid. It expects .base->0 is a
/// backslash (\\), as this is the only valid start of an
/// escape sequence.
///
/// [Link]: https://yaml.org/spec/1.2/spec.html#c-escape
pub(in crate::scanner) fn flow_unescape(base: &str, scratch: &mut Vec<u8>) -> Result<usize>
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

/// Unescape a percent encoded UTF8 tag escape sequence as
/// defined in [Section 5.6][Link], writing the code point
/// to the scratch, returning the length of .base consumed.
///
/// [Link]: https://yaml.org/spec/1.2/spec.html#ns-uri-char
pub(in crate::scanner) fn tag_uri_unescape(
    base: &str,
    scratch: &mut Vec<u8>,
    _directive: bool,
) -> Result<usize>
{
    let mut buffer = base;
    let mut codepoint_len: i8 = 0;

    while {
        if buffer.len() < 3
        {
            return Err(ScanError::UnexpectedEOF);
        }

        if !(check!(~buffer => b'%') && isHex!(~buffer, 1) && isHex!(~buffer, 2))
        {
            return Err(ScanError::UnknownEscape);
        }

        // Safety: we just checked that there are at least three
        // bytes in the buffer
        let octet: u8 = (as_hex(buffer.as_bytes()[1]) << 4) + as_hex(buffer.as_bytes()[2]);

        match codepoint_len
        {
            // First time through, determine how many octets this codepoint has
            0 =>
            {
                codepoint_len = match octet
                {
                    o if (o & 0x80) == 0x00 => 1,
                    o if (o & 0xE0) == 0xC0 => 2,
                    o if (o & 0xF0) == 0xE0 => 3,
                    o if (o & 0xF8) == 0xF0 => 4,
                    _ => return Err(ScanError::UnknownEscape),
                }
            },
            // Else ensure that the trailing octet is valid
            _ =>
            {
                if (octet & 0xC0) != 0x80
                {
                    return Err(ScanError::UnknownEscape);
                }
            },
        }

        scratch.push(octet);
        codepoint_len -= 1;
        advance!(buffer, 3);

        codepoint_len > 0
    }
    {}

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
    if b >= b'A' && b <= b'F'
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
    }
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
    use anyhow::{anyhow, bail};
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

    #[test]
    fn tag_uri_unescape_codepoint() -> TestResult
    {
        let data = &[
            r#"%C2%85"#,
            r#"%c5%b4"#,
            r#"%E2%B1%bf"#,
            r#"%E2%B8%BF"#,
            r#"%f0%90%8f%95"#,
            r#"%F0%90%AD%81"#,
        ];
        let expected: &[&[u8]] = &[
            &[0xC2, 0x85],
            &[0xC5, 0xB4],
            &[0xE2, 0xB1, 0xBF],
            &[0xE2, 0xB8, 0xBF],
            &[0xF0, 0x90, 0x8F, 0x95],
            &[0xF0, 0x90, 0xAD, 0x81],
        ];
        let scratch = &mut Vec::new();

        assert_eq!(
            data.len(),
            expected.len(),
            "test data and expected data are not the same length"
        );

        for (i, (&t, &e)) in data.into_iter().zip(expected).enumerate()
        {
            scratch.clear();

            let consumed = tag_uri_unescape(t, scratch, true)
                .map_err(|e| anyhow!("on iteration {}, test errored with {}", i, e))?;

            assert_eq!(
                &*scratch, e,
                "on iteration {}, expected byte sequence {:?}, got {:?}",
                i, e, &*scratch
            );

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

    #[test]
    fn tag_uri_unescape_eof() -> TestResult
    {
        let data = r#"%C2%8"#;
        let scratch = &mut Vec::new();
        let expected = ScanError::UnexpectedEOF;

        match tag_uri_unescape(data, scratch, true)
        {
            Err(e) if e == expected => Ok(()),

            Err(e) => bail!("expected error: {}, got different error: {}", expected, e),
            Ok(amt) => bail!(
                "expected error: {}, got unexpected value: {}",
                expected,
                amt
            ),
        }
    }

    #[test]
    fn tag_uri_unescape_invalid() -> TestResult
    {
        let data = r#"\xC285"#;
        let scratch = &mut Vec::new();
        let expected = ScanError::UnknownEscape;

        match tag_uri_unescape(data, scratch, true)
        {
            Err(e) if e == expected => Ok(()),

            Err(e) => bail!("expected error: {}, got different error: {}", expected, e),
            Ok(amt) => bail!(
                "expected error: {}, got unexpected value: {}",
                expected,
                amt
            ),
        }
    }
}
