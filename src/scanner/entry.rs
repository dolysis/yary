use std::cmp::Ordering;

use crate::token::Token;

/// A wrapper around a token containing a custom Ord impl
/// based on the token's position in the buffer.
///
/// Note that this wrapper *does not* compare tokens, so if
/// you desire that ensure that you compare them directly
#[derive(Debug)]
pub(crate) struct TokenEntry<'de>
{
    pub token: Token<'de>,
    read_at:   usize,
}

impl<'de> TokenEntry<'de>
{
    pub fn new(token: Token<'de>, read_at: usize) -> Self
    {
        Self { token, read_at }
    }

    pub fn read_at(&self) -> usize
    {
        self.read_at
    }

    pub fn into_token(self) -> Token<'de>
    {
        self.token
    }
}

impl<'de> PartialEq for TokenEntry<'de>
{
    fn eq(&self, other: &Self) -> bool
    {
        self.read_at.eq(&other.read_at)
    }
}

impl<'de> Eq for TokenEntry<'de> {}

impl<'de> PartialOrd for TokenEntry<'de>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

impl<'de> Ord for TokenEntry<'de>
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        self.read_at.cmp(&other.read_at)
    }
}
