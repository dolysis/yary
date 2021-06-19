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
            None => false
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
            Some(buffer) => check!(@priv buffer => $( $match )|+ else $error),
            _ => Err($error)
        }
    };
    (@priv $buffer:expr => $( $match:tt )|+, else $error:expr) => {
        match $buffer {
            $( check!(@ptn $match) )|+ => Ok(()),
            [] => Err($crate::scanner::error::ScanError::UnexpectedEOF),
            _ => Err($error)
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
}

/// Check if the byte (@ .offset) is a line break
///
/// Modifiers
///     ~ .buffer := .buffer.as_bytes()
///
/// Variants
///     /1 .buffer := /2 .buffer, 0
///     /2 .buffer, .offset
macro_rules! isBreak {
    (~ $buffer:expr $(, $offset:expr )? ) => {
        isBreak!($buffer.as_bytes() $(, $offset )? )
    };
    ($buffer:expr $(, $offset:expr )? ) => {
        check!($buffer $(, $offset)? =>
            b'\r'                                   /* CR   #xD     */
            | b'\n'                                 /* LF   #xA     */
            | [b'\xC2', b'\x85', ..]                /* NEL  #x85    */
            | [b'\xE2', b'\x80', b'\xA8', ..]       /* LS   #x2028  */
            | [b'\xE2', b'\x80', b'\xA9', ..]       /* PS   #x2029  */
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
macro_rules! isBlank {
    (~ $buffer:expr $(, $offset:expr )? ) => {
        isBlank!($buffer.as_bytes() $(, $offset )? )
    };
    ($buffer:expr $(, $offset:expr )? ) => {
        check!($buffer $(, $offset)? => b' ' | b'\t')
    };
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
macro_rules! isBlankZ {
    (~ $buffer:expr $(, $offset:expr )? ) => {
        isBlankZ!($buffer.as_bytes() $(, $offset )? )
    };
    ($buffer:expr $(, $offset:expr )? ) => {
        isBlank!($buffer $(, $offset)?)
            || isBreak!($buffer $(, $offset)?)
            || check!($buffer $(, $offset)? => [])
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
    fn scanner_macro_isBlankZ()
    {
        let data: [&[char]; 3] = [&BLANK_CHARS, &BREAK_CHARS, &END_OF_FILE];

        for brk in data.iter().flat_map(|a| *a)
        {
            let mut c = [0; 4];
            let b = brk.encode_utf8(&mut c);

            let test = dbg!(isBlankZ!(~b), isBlankZ!(b.as_bytes()));

            assert!(test.0 && test.1);
        }
    }

    const BREAK_CHARS: [char; 5] = ['\r', '\n', '\u{0085}', '\u{2028}', '\u{2029}'];
    const BLANK_CHARS: [char; 2] = [' ', '\t'];
    const END_OF_FILE: [char; 0] = [];
}
