/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

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
            flag::{Flags, O_EXTENDABLE},
            tests::TEST_FLAGS as PARENT_FLAGS,
        },
        token::Token,
    };

    pub(super) type TestResult = anyhow::Result<()>;

    // Note we expressly remove O_EXTENDABLE, as the tests in
    // this module are not designed to handle Extend errors.
    pub(super) const TEST_FLAGS: Flags = PARENT_FLAGS.difference(O_EXTENDABLE);

    /// Process any deferred Tokens
    pub(super) fn normalize<'de>(
        (maybe, amt): (MaybeToken<'de>, usize),
    ) -> Result<(Token<'de>, usize)>
    {
        Ok((maybe.into_token()?, amt))
    }
}
