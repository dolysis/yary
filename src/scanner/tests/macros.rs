/// Macro for asserting token streams
/// Used as: events!(Scanner => <sigil> <expected> [=>
/// <message>] [, ..]) Where:
///     <sigil>     '|' for a Token, or '@' for an
/// Option<Token>     <expected>  Either Token or
/// Option<Token>     <message>   A message to print on
/// failure
macro_rules! tokens {
    ($scanner:expr => $($id:tt $expected:expr $(=> $msg:tt)?),+ ) => {
        let mut scratch = Vec::new();
        let mut iter = crate::scanner::ScanIter::new(&mut $scanner, &mut scratch);

        let mut f = || -> std::result::Result<(), ::anyhow::Error> {

            $( tokens!(@unwrap $id iter => $expected $(=> $msg)? ); )+

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
        assert_eq!($scanner.next().transpose()?, $expected $(, $msg)? )
    };
    (@unwrap > $scanner:expr => $expected:expr $(=> $msg:tt)? ) => {
        let event = match $scanner
            .next()
            .map(|res| res.map(|r| r.into_inner()))
        {
                Some(r) => r,
                None => anyhow::bail!("Unexpected end of tokens, was expecting: {:?} ~{}", $expected, $scanner.inner.buffer),

        };
        assert_eq!(event, $expected $(, $msg)? )
    };
    // Forward to option assert any unknown sigils
    (@unwrap $any:tt $scanner:expr => $expected:expr $(=> $msg:tt)? ) => {
        tokens!(@unwrap @ $scanner:expr => $expected:expr $(=> $msg)? )
    };
    // Variant for token assert, no message
    (@token $scanner:expr => $expected:expr) => {
        let event = match $scanner.next()
        {
            Some(r) => match r
            {
                Ok(r) => r,
                Err(e) => anyhow::bail!("{} ~{}", e, $scanner.inner.buffer),
            }
            None => anyhow::bail!("Unexpected end of tokens, was expecting: {:?} ~{}", $expected, $scanner.inner.buffer)
        };

        assert_eq!(event, $expected)
    };
    // Variant for token assert, with message
    (@token $scanner:expr => $expected:expr, $msg:tt) => {
        let event = match $scanner.next()
        {
            Some(r) => match r
            {
                Ok(r) => r,
                Err(e) => anyhow::bail!("{} ~{}", e, $scanner.inner.buffer),
            },
            None => anyhow::bail!("Unexpected end of tokens, {}: {:?} ~{}", $msg, $expected, $scanner.inner.buffer)
        };

        assert_eq!(event, $expected, $msg)
    };
}
