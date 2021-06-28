//! This module contains the various macros used by
//! lib/scanner.

/// Rebinds .buffer's binding .amount forward, optionally
/// taking a .var to add .amount to
///
/// Modifiers
///     <- .buffer := return .buffer->0...amount
///
/// Variants
///     /1 .buffer, .amount
///     /2 .buffer, .amount, .var
macro_rules! advance {
    ($buffer:expr, $amount:expr $(, $var:ident )? ) => {
        let (_, rest) = $buffer.split_at($amount);

        $( advance!(@update $var, $amount); )?

        $buffer = rest
    };
    (<- $buffer:expr, $amount:expr $(, $var:ident )? ) => {{
        let (cut, rest) = $buffer.split_at($amount);

        $buffer = rest;

        $( advance!(@update $var, $amount) )?

        cut
    }};

    (@update $( $var:ident, $amount:expr)? ) => {
          $({ $var += $amount } )?
    };
}

/// New cow pointer from the given expr
macro_rules! cow {
    ($from:expr) => {
        std::borrow::Cow::from($from)
    };
}

/// Check the .buffer (@ .offset) matches the given
/// .pattern, optionally returning an .error.
///
/// Note that the error path is special cased to return an
/// UnexpectedEOF if it encounters an empty slice, although
/// this can be overridden by expressly including an empty
/// pattern ([]) in your .pattern
///
/// Modifiers
///     ~ .buffer := .buffer.as_bytes()
///
/// Variants
///     /1 .buffer => .pattern := /2 .buffer, 0 => .pattern
///     /2 .buffer, .offset => .pattern
///     /3 .buffer => .pattern, else .error
///             := /4 .buffer, 0 => .pattern else .error
///     /4 .buffer, .offset => .pattern, else .error
macro_rules! check {
    (~ $buffer:expr $(, $offset:expr )? => $( $match:tt )|+ $(, else $error:expr)? ) => {
        check!(@priv $buffer.as_bytes() $(, $offset )? => $( $match )|+ $(, else $error)?)
    };

    ($buffer:expr $(, $offset:expr )? => $( $match:tt )|+ $(, else $error:expr)? ) => {
        check!(@priv $buffer $(, $offset )? => $( $match )|+ $(, else $error)?)
    };

    /* Private variants */
    (@priv $buffer:expr, $offset:expr => $( $match:tt )|+) => {
        match $buffer.get($offset..) {
            Some(buffer) => check!(@priv buffer => $( $match )|+),
            None => check!(@eofck $( $match )|+ ),
        }
    };
    (@priv $buffer:expr => $( $match:tt )|+) => {
        match $buffer {
            $( check!(@ptn $match) )|+ => true,
            _ => false
        }
    };
    (@priv $buffer:expr, $offset:expr => $( $match:tt )|+, else $error:expr) => {
        match $buffer.get($offset..) {
            Some(buffer) => check!(@priv buffer => $( $match )|+, else $error),
            None if check!(@eofck $( $match )|+ ) => Ok(()),
            _ => Err($crate::scanner::error::ScanError::UnexpectedEOF),
        }
    };
    (@priv $buffer:expr => $( $match:tt )|+, else $error:expr) => {
        match $buffer {
            $( check!(@ptn $match) )|+ => Ok(()),
            [] => Err($crate::scanner::error::ScanError::UnexpectedEOF),
            _ => Err($error),
        }
    };

    // Note we use macro path rules to first try matching the given
    // token as a literal, e.g a b'_', then try it as a pattern
    (@ptn $byte:literal) => {
        [$byte, ..]
    };
    (@ptn $match:pat) => {
        $match
    };

    // When indexing to an out of bounds .offset, we mostly want
    // to return false, however if the caller is checking for an
    // out of bounds (e.g a [] pattern) we must special case this
    // and return true
    (@eofck $( $match:tt )|+) => {{
        #[allow(unused_mut)]
        let mut checking_newline = false;
        $( check!(@eofck &mut checking_newline, $match); )+

        checking_newline
    }};
    // _If and only if_ there is an empty slice pattern, set
    // checking_newline to true as the caller wants to positively
    // check for EOF
    (@eofck $is_checking:expr, []) => {
        *$is_checking = true
    };
    (@eofck $is_checking:expr, $_:literal) => {
    };
    (@eofck $is_checking:expr, $_:pat) => {
    };
}

/// Check if the byte (@ .offset) is a line break
///
/// Modifiers
///     ~ .buffer := .buffer.as_bytes()
///
/// Variants
///     /1 .buffer := /2 .buffer, 0
///     /2 .buffer, .offset
///     /3 .buffer, else .error
///             := /4 .buffer, 0, else .error
///     /4 .buffer, .offset, else .error
macro_rules! isBreak {
    (~ $buffer:expr $(, $offset:expr )? $(, else $error:expr )? ) => {
        isBreak!($buffer.as_bytes() $(, $offset )? $(, else $error)?)
    };
    ($buffer:expr $(, $offset:expr )? $(, else $error:expr )? ) => {
        check!($buffer $(, $offset)? =>
            b'\r'                                   /* CR   #xD     */
            | b'\n'                                 /* LF   #xA     */
            | [b'\xC2', b'\x85', ..]                /* NEL  #x85    */
            | [b'\xE2', b'\x80', b'\xA8', ..]       /* LS   #x2028  */
            | [b'\xE2', b'\x80', b'\xA9', ..]       /* PS   #x2029  */
            $(, else $error)?
        )
    };
}

/// Check if the byte (@ .offset) is a space or tab
///
/// Modifiers:
///     ~ .buffer := .buffer.as_bytes()
///
/// Variants:
///     /1 .buffer := /2 .buffer, 0
///     /2 .buffer, .offset
///     /3 .buffer, else .error
///             := /4 .buffer, 0, else .error
///     /4 .buffer, .offset, else .error
macro_rules! isBlank {
    (~ $buffer:expr $(, $offset:expr )? $(, else $error:expr )? ) => {
        isBlank!($buffer.as_bytes() $(, $offset )? $(, else $error )? )
    };
    ($buffer:expr $(, $offset:expr )? $(, else $error:expr )? ) => {
        check!($buffer $(, $offset)? => b' ' | b'\t' $(, else $error )? )
    };
}

/// Check if the byte (@ .offset) is a space, tab or
/// line break
///
/// Modifiers:
///     ~ .buffer := .buffer.as_bytes()
///
/// Variants:
///     /1 .buffer := /2 .buffer, 0
///     /2 .buffer, .offset
///     /3 .buffer, else .error
///             := /4 .buffer, 0, else .error
///     /4 .buffer, .offset, else .error
macro_rules! isWhiteSpace {
    (~ $buffer:expr $(, $offset:expr )? $(, else $error:expr )? ) => {
        isWhiteSpace!($buffer.as_bytes() $(, $offset )? $(, else $error)? )
    };
    ($buffer:expr $(, $offset:expr )? $(, else $error:expr )? ) => {
        isWhiteSpace!(@priv $buffer $(, $offset )? $(, else $error)? )
    };
    (@priv $buffer:expr $(, $offset:expr )? ) => {
        isBlank!($buffer $(, $offset)?)
            || isBreak!($buffer $(, $offset)?)
    };
    (@priv $buffer:expr $(, $offset:expr )?, else $error:expr ) => {
        isBlank!($buffer $(, $offset)?, else $error)
            .or_else(|_| isBreak!($buffer $(, $offset)?, else $error))
    }
}

/// Check if the byte (@ .offset) is a space, tab, line
/// break or if .buffer is empty
///
/// Modifiers:
///     ~ .buffer := .buffer.as_bytes()
///
/// Variants:
///     /1 .buffer := /2 .buffer, 0
///     /2 .buffer, .offset
macro_rules! isWhiteSpaceZ {
    (~ $buffer:expr $(, $offset:expr )? ) => {
        isWhiteSpaceZ!($buffer.as_bytes() $(, $offset )? )
    };
    ($buffer:expr $(, $offset:expr )? ) => {
        isBlank!($buffer $(, $offset)?)
            || isBreak!($buffer $(, $offset)?)
            || check!($buffer $(, $offset)? => [])
    };
}

/// Checks if byte (@ .offset) in .buffer is hexadecimal
///
/// Modifiers:
///     ~ .buffer := .buffer.as_bytes()
///
/// Variants:
///     /1 .buffer := /2 .buffer, 0
///     /2 .buffer, .offset
///     /3 .buffer, else .error
///             := /4 .buffer, 0, else .error
///     /4 .buffer, .offset, else .error
macro_rules! isHex {
    (~ $buffer:expr $(, $offset:expr )? $(, else $error:expr )? ) => {
        isHex!($buffer.as_bytes() $(, $offset)? $(, else $error)? )
    };
    ($buffer:expr $(, $offset:expr )? $(, else $error:expr)? ) => {
        check!($buffer $(, $offset)? =>
                [b'0'..=b'9', ..] | [b'A'..=b'F', ..] | [b'a'..=b'f', ..]
                $(, else $error )?
            )
    };
}

#[cfg(test)]
mod tests
{
    #![allow(non_snake_case)]

    #[test]
    fn scanner_macro_isBreak()
    {
        let data = BREAK_CHARS;

        for brk in &data
        {
            let mut c = [0; 4];
            let b = brk.encode_utf8(&mut c);

            let test = dbg!(isBreak!(~b), isBreak!(b.as_bytes()));

            assert!(test.0 && test.1);
        }
    }

    #[test]
    fn scanner_macro_isBreak_offset()
    {
        let data = BREAK_CHARS;

        for brk in &data
        {
            let mut c = [0; 8];
            brk.encode_utf8(&mut c[4..]);
            let b = std::str::from_utf8(&c).expect("valid UTF8");

            let test = dbg!(isBreak!(~b, 4), isBreak!(b.as_bytes(), 4));

            assert!(test.0 && test.1);
        }
    }

    #[test]
    fn scanner_macro_isBlank()
    {
        let data = BLANK_CHARS;

        for brk in &data
        {
            let mut c = [0; 4];
            let b = brk.encode_utf8(&mut c);

            let test = dbg!(isBlank!(~b), isBlank!(b.as_bytes()));

            assert!(test.0 && test.1);
        }
    }

    #[test]
    fn scanner_macro_isBlank_offset()
    {
        let data = BLANK_CHARS;

        for brk in &data
        {
            let mut c = [0; 8];
            brk.encode_utf8(&mut c[4..]);
            let b = std::str::from_utf8(&c).expect("valid UTF8");

            let test = dbg!(isBlank!(~b, 4), isBlank!(b.as_bytes(), 4));

            assert!(test.0 && test.1);
        }
    }

    #[test]
    fn scanner_macro_isWhiteSpaceZ()
    {
        let data: [&[char]; 2] = [&BLANK_CHARS, &BREAK_CHARS];

        for brk in data.iter().flat_map(|a| *a)
        {
            let mut c = [0; 4];
            let b = brk.encode_utf8(&mut c);

            let test = dbg!(isWhiteSpaceZ!(~b), isWhiteSpaceZ!(b.as_bytes()));

            assert!(test.0 && test.1);
        }

        let empty = "";

        let test = dbg!((isWhiteSpaceZ!(~empty), isWhiteSpaceZ!(empty.as_bytes())));

        assert!(test.0 && test.1);
    }

    #[test]
    fn scanner_macro_isWhiteSpaceZ_offset()
    {
        let data: [&[char]; 2] = [&BLANK_CHARS, &BREAK_CHARS];

        for brk in data.iter().flat_map(|a| *a)
        {
            let mut c = [0; 8];
            brk.encode_utf8(&mut c[4..]);
            let b = std::str::from_utf8(&c).expect("valid UTF8");

            let test = dbg!(isWhiteSpaceZ!(~b, 4), isWhiteSpaceZ!(b.as_bytes(), 4));

            assert!(test.0 && test.1);
        }

        let empty = "    ";

        let test = dbg!((
            isWhiteSpaceZ!(~empty, 5),
            isWhiteSpaceZ!(empty.as_bytes(), 5)
        ));

        assert!(test.0 && test.1);
    }

    const BREAK_CHARS: [char; 5] = ['\r', '\n', '\u{0085}', '\u{2028}', '\u{2029}'];
    const BLANK_CHARS: [char; 2] = [' ', '\t'];
}
