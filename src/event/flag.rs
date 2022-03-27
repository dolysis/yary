/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Configuration flags used to control aspects of the Event
//! production pipeline.
//!
//! Read the documentation on each flag's `O_*` constant for
//! an explanation of it's purpose.
//!
//! ```
//! # use yary::event::flag::*;
//! // You can use bitwise operators
//! let bitwise = O_NIL | O_LAZY;
//!
//! // Or if you prefer, method chaining
//! let chaining = Flags::new().no_lazy().nil().lazy();
//!
//! assert_eq!(bitwise, chaining);
//! ```

use bitflags::bitflags;

use crate::scanner;

/// An empty, zeroed flag set. This is the default set,
/// with all other flags disabled.
pub const O_NIL: Flags = Flags::empty();

/// Set to lazily process the underlying byte stream.
///
/// In particular, scalars events will not be fully
/// processed, instead being returned as
/// [ScalarLike::Lazy] variants.
///
/// This means that any allocations, escape parsing
/// or line joins will be deferred until the caller
/// explicitly requests the token. This also applies to
/// errors in the scalar itself, which will not be
/// caught until the caller requests the token!
///
/// This option is useful if the caller is expecting to
/// care about only a small portion of the total scalars
/// in the stream, as it allows us to defer significant
/// portions of the computation, potentially forever if
/// the caller decides to never materialize the
/// underlying scalar.
///
/// [ScalarLike::Lazy]: super::types::ScalarLike::Lazy
pub const O_LAZY: Flags = Flags::LAZY;

impl Flags
{
    /// Instantiates a new, empty flag set
    pub const fn new() -> Self
    {
        O_NIL
    }

    /// Nulls the flag set, resetting it to empty
    pub const fn nil(self) -> Self
    {
        O_NIL
    }

    /// Applies [`O_LAZY`] to this flag set
    pub const fn lazy(self) -> Self
    {
        self.union(O_LAZY)
    }

    /// Removes [`O_LAZY`] from this flag set
    pub const fn no_lazy(self) -> Self
    {
        self.difference(O_LAZY)
    }
}

/// Convert from public flags to Scanner specific flags
pub(crate) const fn as_scanner(f: Flags) -> scanner::flag::Flags
{
    use scanner::flag;

    let mut flags = flag::O_ZEROED;

    if f.contains(O_LAZY)
    {
        flags = flags.union(flag::O_LAZY);
    }

    flags
}

bitflags! {
    /// Controls aspects of [Events] behaviors, read each flag for more information.
    ///
    /// [Events]: super::Events
    #[derive(Default)]
    pub struct Flags: u32 {
        /// See [`O_LAZY`]
        const LAZY          = 0b00000001;
    }
}
