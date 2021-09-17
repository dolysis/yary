/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use bitflags::bitflags;

/// An empty, zeroed flag set. This is the default set, with
/// all other flags disabled.
pub const O_ZEROED: Flags = Flags::empty();
/// Hints to the Scanner if the given byte slice can be
/// extended. Typically used when processing data in chunks,
/// or in circumstances when there may be more data in the
/// future.
///
/// If this flag is set the Scanner will return a
/// ScanError::Extend if the byte stream terminates before a
/// token can be scanned.
pub const O_EXTENDABLE: Flags = Flags::EXTENDABLE;
/// Sets the Scanner to lazily process the underlying byte
/// stream.
///
/// In particular, the Scanner will not fully process
/// scalars, only locating the start and end markers in the
/// stream. This means that any allocations, escape parsing
/// or line joins will be deferred until the caller
/// explicitly requests the token. This _also applies to
/// errors_ in the scalar itself, which will not be caught
/// until the caller requests the token!
pub const O_LAZY: Flags = Flags::LAZY;

bitflags! {
    /// Directives controlling various behaviors of the Scanner,
    /// see each O_ variant for an explanation of how each works
    #[derive(Default)]
    pub struct Flags: u32 {
        const EXTENDABLE    = 0b00000001;
        const LAZY          = 0b00000010;
    }
}
