/// Moves head in $buffer $amount forward
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

/// Check the buffer for $byte matches at $pos, optionally
/// returning an error Note that the error path is special
/// cased to return an UnexpectedEOF if it encounters an
/// empty slice
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

/// Check if the char (@offset) is a line break
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

/// Check if the char (@offset) is a space or tab
macro_rules! isBlank {
    (~ $buffer:expr $(, $offset:expr )? ) => {
        isBlank!($buffer.as_bytes() $(, $offset )? )
    };
    ($buffer:expr $(, $offset:expr )? ) => {
        check!($buffer $(, $offset)? => b' ' | b'\t')
    };
}

/// Check if the char (@offset) is a space, tab or line
/// break
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
