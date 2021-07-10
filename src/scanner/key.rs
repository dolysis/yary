use super::scalar::flow::ScalarRange;
use crate::{
    scanner::error::ScanResult as Result,
    token::{Ref, Token},
};

#[derive(Debug, Clone)]
pub(in crate::scanner) struct Key
{
    next:  Option<NextKey>,
    state: Option<KeyState>,

    pub mark: usize,
    pub line: usize,
}

impl Key
{
    pub fn new(mark: usize, line: usize) -> Self
    {
        Self {
            mark,
            line,
            state: None,
            next: None,
        }
    }

    /// A key is possible / .required at the current stream
    /// position
    pub fn possible(&mut self, required: bool)
    {
        self.next = match required
        {
            true => NextKey::Required,
            false => NextKey::Possible,
        }
        .into();
    }

    /// A key is impossible / illegal at the current stream
    /// position
    pub fn impossible(&mut self)
    {
        self.next = Some(NextKey::Disallowed)
    }

    /// Is a key allowed at the current position?
    pub fn allowed(&self) -> bool
    {
        self.next.as_ref().map(|s| s.allowed()).unwrap_or(false)
    }

    /// Is a key required at the current position?
    pub fn required(&self) -> bool
    {
        self.next.as_ref().map(|s| s.required()).unwrap_or(false)
    }

    /// Save a scalar token, starting a token sequence
    pub fn save(&mut self, r: ScalarRange, advance: usize)
    {
        self.state = Some(KeyState::new(r, advance))
    }

    pub fn has_tokens(&self) -> bool
    {
        self.state.is_some()
    }

    pub fn next_token<'b, 'c>(
        &mut self,
        base: &'b str,
        scratch: &'c mut Vec<u8>,
    ) -> Result<Option<(Ref<'b, 'c>, usize)>>
    {
        let state = match self.state.take()
        {
            Some(state) => state,
            None => return Ok(None),
        };

        match state.next_state(base, scratch)?
        {
            (state @ Some(_), token, amt) =>
            {
                self.state = state;

                Ok(Some((token, amt)))
            },
            (None, token, amt) => Ok(Some((token, amt))),
        }
    }
}

impl Default for Key
{
    fn default() -> Self
    {
        Self::new(0, 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::scanner) enum NextKey
{
    Disallowed,
    Possible,
    Required,
}

impl NextKey
{
    fn allowed(&self) -> bool
    {
        matches!(self, Self::Possible | Self::Required)
    }

    fn required(&self) -> bool
    {
        matches!(self, Self::Required)
    }
}

impl Default for NextKey
{
    fn default() -> Self
    {
        Self::Disallowed
    }
}

#[derive(Debug, Clone)]
pub(in crate::scanner) enum KeyState
{
    Start(ScalarRange, usize),
    KeyYielded(ScalarRange, usize),
}

impl KeyState
{
    pub fn new(r: ScalarRange, advance: usize) -> Self
    {
        Self::Start(r, advance)
    }

    pub fn next_state<'b, 'c>(
        self,
        base: &'b str,
        scratch: &'c mut Vec<u8>,
    ) -> Result<(Option<Self>, Ref<'b, 'c>, usize)>
    {
        match self
        {
            Self::Start(r, amt) => Ok((Some(Self::KeyYielded(r, amt)), Token::Key.borrowed(), 0)),
            Self::KeyYielded(r, amt) => Ok((None, r.into_token(base, scratch)?, amt)),
        }
    }

    pub fn into_inner(self) -> (ScalarRange, usize)
    {
        match self
        {
            Self::Start(r, amt) | Self::KeyYielded(r, amt) => (r, amt),
        }
    }
}
