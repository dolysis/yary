use super::{
    error::{ScanError, ScanResult as Result},
    stats::MStats,
    ALIAS, ANCHOR,
};
use crate::token::Token;

/// Scan an anchor or alias from the underlying .buffer
/// returning the relevant Token
pub(in crate::scanner) fn scan_anchor<'de>(
    buffer: &mut &'de str,
    stats: &mut MStats,
    kind: &AnchorKind,
) -> Result<Token<'de>>
{
    advance!(*buffer, :stats, 1);

    // *anchor 'rest of the line'
    //  ^^^^^^
    let anchor = take_while(buffer.as_bytes(), u8::is_ascii_alphanumeric);

    let anchor = advance!(<- *buffer, :stats, anchor.len());

    // anchor name cannot be empty, must contain >= 1
    // alphanumeric character
    if anchor.is_empty()
    {
        return Err(ScanError::InvalidAnchorName);
    }

    // *anchor 'rest of the line'
    //        ^
    // There does not necessarily need to be a whitespace so we
    // also check against a list of valid starting
    // tokens
    check!(~buffer
        => b' ' | b'\n' | b'?' | b',' | b']' | b'}' | b'%' | b'@' | b'`',
        else ScanError::InvalidAnchorName
    )?;

    let token = match kind
    {
        AnchorKind::Alias => Token::Alias(cow!(anchor)),
        AnchorKind::Anchor => Token::Anchor(cow!(anchor)),
    };

    Ok(token)
}

/// Representation of a YAML anchor (&) or alias (*) node
/// tag
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(in crate::scanner) enum AnchorKind
{
    Anchor,
    Alias,
}

impl AnchorKind
{
    /// Fallibly determine whether an anchor or alias
    /// starts from the given .byte
    pub fn new(byte: &u8) -> Option<Self>
    {
        let s = match byte
        {
            &ALIAS => Self::Alias,
            &ANCHOR => Self::Anchor,
            _ => return None,
        };

        Some(s)
    }
}

fn take_while<F>(b: &[u8], f: F) -> &[u8]
where
    F: Fn(&u8) -> bool,
{
    let mut index = 0;

    loop
    {
        match b.get(index)
        {
            Some(b) if f(b) => index += 1,
            _ => return &b[..index],
        }
    }
}
