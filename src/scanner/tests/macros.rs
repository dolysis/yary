/// Macro for asserting token streams
/// Used as: events!(Scanner => <sigil> <expected> [=> <message>] [, ..])
/// Where:
///     <sigil>     '|' for a Token, or '@' for an Option<Token>
///     <expected>  Either Token or Option<Token>
///     <message>   A message to print on failure
macro_rules! tokens {
    ($scanner:expr => $($id:tt $expected:expr $(=> $msg:tt)?),+ ) => {
        $( tokens!(@unwrap $id $scanner => $expected $(=> $msg)? ) );+
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
        assert_eq!($scanner.next().expect("Unexpected end of events"), $expected)
    };
    // Variant for token assert, no with message
    (@token $scanner:expr => $expected:expr, $msg:tt) => {
        let event = $scanner.next().expect("Unexpected end of events");

        assert_eq!(event, $expected, $msg)
    };
}
