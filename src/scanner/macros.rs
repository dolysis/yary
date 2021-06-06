/// Moves head in $buffer $amount forward
macro_rules! advance {
    ($buffer:expr, $amount:expr) => {
        let (_, rest) = $buffer.split_at($amount);

        $buffer = rest
    };
    (<- $buffer:expr, $amount:expr) => {{
        let (cut, rest) = $buffer.split_at($amount);

        $buffer = rest;

        cut
    }};
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
    ($buffer:expr, $(@$pos:expr,)? is $( $byte:pat )|+ $(, else $error:expr)? ) => {
        {
            let b = match $buffer$([$pos..])? {
                [] => Err(false),
                $([$byte, ..])|+ => Ok(true),
                _ => Ok(false)
            };

            check!(@priv b $(=> $error)? )
        }
    };
    ($buffer:expr, $(@$pos:expr,)? not $( $byte:pat )|+ $(, else $error:expr)? ) => {
        {
            let b = match $buffer$([$pos..])? {
                [] => Err(true),
                $([$byte, ..])|+ => Ok(false),
                _ => Ok(true)
            };

            check!(@priv b $(=> $error)? )
        }
    };
    (@priv $bool:expr) => {
        match $bool {
            Ok(b) | Err(b) => b
        }
    };
    (@priv $bool:expr => $error:expr) => {
        match $bool {
            Ok(true) => Ok(()),
            Ok(false) => Err($error),
            Err(_) => Err($crate::scanner::error::ScanError::UnexpectedEOF),
        }
    }
}
