/// Macro for asserting token streams
/// Used as: events!(Scanner => <sigil> <expected> [=> <message>] [, ..])
/// Where:
///     <sigil>     '|' for a Token, or '@' for an Option<Token>
///     <expected>  Either Token or Option<Token>
///     <message>   A message to print on failure
macro_rules! tokens {
    ($scanner:expr => $($id:tt $expected:expr $(=> $msg:tt)?),+ ) => {
        let mut f = || -> std::result::Result<(), ::anyhow::Error> {
            $( tokens!(@unwrap $id $scanner => $expected $(=> $msg)? ); )+

            Ok(())
        };

        if let Err(e) = f() {
            panic!("tokens! error: {}", e)
        }
    };

    // <-- PRIVATE VARIANTS -->

    // Forward to the @token variants, with/without a message
    (@unwrap | $scanner:expr => $expected:expr $(=> $msg:tt)? ) => {
        tokens!(@token $scanner => $expected $(, $msg)? )
    };
    // Variant for option assert
    (@unwrap @ $scanner:expr => $expected:expr $(=> $msg:tt)? ) => {
        assert_eq!($scanner.next(), $expected $(, $msg)? )
    };
    // Forward to option assert any unknown sigils
    (@unwrap $any:tt $scanner:expr => $expected:expr $(=> $msg:tt)? ) => {
        tokens!(@unwrap @ $scanner:expr => $expected:expr $(=> $msg)? )
    };
    // Variant for token assert, no message
    (@token $scanner:expr => $expected:expr) => {
        let event = $scanner
            .next()
            .ok_or_else(
                || anyhow::anyhow!("Unexpected end of tokens, was expecting: {:?} ~{}", $expected, $scanner.buffer)
            )?;

        assert_eq!(event, $expected)
    };
    // Variant for token assert, no with message
    (@token $scanner:expr => $expected:expr, $msg:tt) => {
        let event = $scanner
            .next()
            .ok_or_else(
                || anyhow::anyhow!("Unexpected end of tokens, {}: {:?} ~{}", $msg, $expected, $scanner.buffer)
            )?;

        assert_eq!(event, $expected, $msg)
    };
}
