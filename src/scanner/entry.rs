/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;

use crate::{
    scanner::{
        error::ScanResult as Result,
        scalar::{block, flow, plain},
    },
    token::Token,
};

/// A wrapper around a token containing a custom Ord impl
/// based on the token's position in the buffer.
///
/// Note that this wrapper *does not* compare tokens, so if
/// you desire that ensure that you compare them directly
#[derive(Debug)]
pub struct TokenEntry<'de>
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

#[derive(Debug, Clone)]
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
            inner: LazyImpl::Flow(inner),
        }
    }
}

impl<'de> From<plain::Deferred<'de>> for Lazy<'de>
{
    fn from(inner: plain::Deferred<'de>) -> Self
    {
        Self {
            inner: LazyImpl::Plain(inner),
        }
    }
}

impl<'de> From<block::Deferred<'de>> for Lazy<'de>
{
    fn from(inner: block::Deferred<'de>) -> Self
    {
        Self {
            inner: LazyImpl::Block(inner),
        }
    }
}

#[derive(Debug, Clone)]
enum LazyImpl<'de>
{
    Flow(flow::Deferred<'de>),
    Plain(plain::Deferred<'de>),
    Block(block::Deferred<'de>),
}

impl<'de> LazyImpl<'de>
{
    pub fn into_token(self) -> Result<Token<'de>>
    {
        match self
        {
            Self::Flow(inner) => inner.into_token(),
            Self::Plain(inner) => inner.into_token(),
            Self::Block(inner) => inner.into_token(),
        }
    }
}
