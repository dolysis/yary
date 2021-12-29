/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

mod error;

pub use error::{ReaderError, ReaderResult};
use private::Sealed;

use crate::{
    queue::Queue,
    reader::error::ReaderResult as Result,
    scanner::{entry::TokenEntry, flag::Flags, Scanner},
};

/// Sealed interface over the functionality that
/// transforms a byte stream into [Token][crate::token::
/// Token]s.
///
/// Note the key feature here is `&'de self`. Namely, an
/// immutable reference through which any internal mutation
/// must not be visible
pub trait Read: std::fmt::Debug + Sealed
{
    /// Drive the .scanner from the byte stream with the
    /// provided .options, placing output into the
    /// .queue
    fn drive<'de>(
        &'de self,
        scanner: &mut Scanner,
        queue: &mut Queue<TokenEntry<'de>>,
        options: Flags,
    ) -> Result<()>;

    /// Hint to the underlying implementation that no live
    /// references exist to any data read below
    /// the given .bound, and that it may unload the given
    /// memory.
    ///
    /// SAFETY:
    ///     It is only safe to call this function after the
    ///     caller has ensured there cannot be any live
    ///     references to content below the provided .bound.
    unsafe fn consume(&self, _bound: usize) -> Result<()>
    {
        Ok(())
    }
}

mod private
{
    pub trait Sealed
    {
    }
}
