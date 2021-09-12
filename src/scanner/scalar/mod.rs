use super::entry::MaybeToken;

pub mod block;
pub mod escape;
pub mod flow;
pub mod plain;

// Generic Into<MaybeToken> closure
fn as_maybe<'de, T>((token, amt): (T, usize)) -> (MaybeToken<'de>, usize)
where
    T: Into<MaybeToken<'de>>,
{
    (token.into(), amt)
}

#[cfg(test)]
mod test_utils
{
    use crate::{
        scanner::{
            entry::MaybeToken,
            error::ScanResult as Result,
            flag::{self, Flags},
        },
        token::Token,
    };

    pub(super) type TestResult = anyhow::Result<()>;

    pub(super) const TEST_OPTS: Flags = flag::O_ZEROED;

    /// Process any deferred Tokens
    pub(super) fn normalize<'de>(
        (maybe, amt): (MaybeToken<'de>, usize),
    ) -> Result<(Token<'de>, usize)>
    {
        Ok((maybe.into_token()?, amt))
    }
}
