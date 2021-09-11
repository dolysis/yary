use std::cmp::Ordering;

use crate::{
    scanner::{error::ScanResult as Result, scalar::flow},
    token::Token,
};

/// A wrapper around a token containing a custom Ord impl
/// based on the token's position in the buffer.
///
/// Note that this wrapper *does not* compare tokens, so if
/// you desire that ensure that you compare them directly
#[derive(Debug)]
pub(crate) struct TokenEntry<'de>
{
    pub wrap: MaybeToken<'de>,
    read_at:  usize,
}

impl<'de> TokenEntry<'de>
{
    pub fn new<T>(token: T, read_at: usize) -> Self
    where
        T: Into<MaybeToken<'de>>,
    {
        Self {
            wrap: token.into(),
            read_at,
        }
    }

    pub fn read_at(&self) -> usize
    {
        self.read_at
    }

    pub fn is_processed(&self) -> bool
    {
        matches!(&self.wrap, MaybeToken::Token(_))
    }

    pub fn into_token(self) -> Result<Token<'de>>
    {
        self.wrap.into_token()
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

#[derive(Debug)]
pub enum MaybeToken<'de>
{
    Token(Token<'de>),
    Deferred(Lazy<'de>),
}

impl<'de> MaybeToken<'de>
{
    pub fn into_token(self) -> Result<Token<'de>>
    {
        match self
        {
            Self::Token(token) => Ok(token),
            Self::Deferred(lazy) => lazy.into_token(),
        }
    }
}

impl<'de> From<Token<'de>> for MaybeToken<'de>
{
    fn from(token: Token<'de>) -> Self
    {
        Self::Token(token)
    }
}

impl<'de, T> From<T> for MaybeToken<'de>
where
    T: Into<Lazy<'de>>,
{
    fn from(t: T) -> Self
    {
        Self::Deferred(t.into())
    }
}

#[derive(Debug)]
pub struct Lazy<'de>
{
    inner: LazyImpl<'de>,
}

impl<'de> Lazy<'de>
{
    pub fn into_token(self) -> Result<Token<'de>>
    {
        self.inner.into_token()
    }
}
impl<'de> From<flow::Deferred<'de>> for Lazy<'de>
{
    fn from(inner: flow::Deferred<'de>) -> Self
    {
        Self {
            inner: LazyImpl::ScalarF(inner),
        }
    }
}

#[derive(Debug)]
enum LazyImpl<'de>
{
    ScalarF(flow::Deferred<'de>),
}

impl<'de> LazyImpl<'de>
{
    pub fn into_token(self) -> Result<Token<'de>>
    {
        match self
        {
            Self::ScalarF(inner) => inner.into_token(),
        }
    }
}
