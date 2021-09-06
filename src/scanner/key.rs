//! ```text
//! The key Scanner subsystem is responsible for managing
//! the state of implicit key searches.
//!
//! First, an explanation of the problem.
//!
//! Given the following YAML:
//!
//!     !!str &anchor 'a key': 'a value'
//!
//! The following tokens should be produced (ignoring ones
//! irrelevant to this example):
//!
//!     Key
//!     Tag('!!', 'str')
//!     Anchor('anchor')
//!     Scalar('a key')
//!     Value
//!     Scalar('a value')
//!
//! Note where the key is produced.
//!
//!     !!str &anchor 'a key': 'a value'
//!    ^
//!    Key
//!
//! The key must always come before any node decorators
//! (tags, anchors, aliases), but unfortunately, YAML
//! doesn't provide any indicator of an implicit key, thus
//! the first time we'll know that a key should be produced
//! is when we hit the Value (':') token here:
//!
//!     !!str &anchor 'a key': 'a value'
//!                          ^
//!                          Value
//!
//! Therefore we need some way to "save" a possible key's
//! position so when/if we hit a Value token we can backfill
//! the Key to its correct location in the token queue. In
//! addition, sometimes a YAML key is required by the spec
//! -- notably when in the block context and at the same
//! indentation -- so we also need to keep track of whether
//! this potential Key is only possible and it's okay if it
//! doesn't exist or if its required and an error if it
//! doesn't exist.
//!
//! So, the solution.
//!
//! Basically boils down to three things:
//!
//! 1. Keeping track of whether we can save a simple key
//!
//!     I.E after hitting one of the decorators, we *don't*
//!     want to overwrite an existing saved position
//!
//! 2. Saving the key's position in the buffer and state
//! 3. Adding the key (or not) to the queue at the correct
//!    position
//!
//! This module manages 2. 1 is managed across the various
//! Scanner functions -- see scanner.simple_key_allowed
//! call sites. While 3. is managed in part by the Queue
//! implementation used by the Scanner; namely that it is a
//! stable min heap... which allows us to use the saved
//! position in the buffer to push the Key to its correct
//! queue position
//! ```

use crate::scanner::stats::MStats;

/// Manages the state for tracking possible implicit Keys
/// that the Scanner might may encounter during runtime
#[derive(Debug, Clone)]
pub(in crate::scanner) struct Key
{
    save: Option<KeySave>,
}

impl Key
{
    pub fn new() -> Self
    {
        Self { save: None }
    }

    /// Save a potential simple key
    pub fn save(&mut self, stats: MStats, required: bool)
    {
        let save = KeySave::new(stats, required);

        self.save = Some(save)
    }

    /// Retrieve a potential simple key
    pub fn saved(&mut self) -> &mut Option<KeySave>
    {
        &mut self.save
    }

    /// Is a simple key possible?
    pub fn possible(&self) -> bool
    {
        self.save
            .as_ref()
            .map_or(false, |saved| saved.key().allowed())
    }

    /// Is a simple key required?
    pub fn required(&self) -> bool
    {
        self.save
            .as_ref()
            .map_or(false, |saved| saved.key().required())
    }
}

impl Default for Key
{
    fn default() -> Self
    {
        Self::new()
    }
}

/// Holds the state of a potential key, specifically its
/// possibility and where in the stream it was encountered
#[derive(Debug, Clone)]
pub(in crate::scanner) struct KeySave
{
    possible: KeyPossible,
    stats:    MStats,
}

impl KeySave
{
    pub fn new(stats: MStats, required: bool) -> Self
    {
        let possible = match required
        {
            true => KeyPossible::Required,
            false => KeyPossible::Yes,
        };

        Self { possible, stats }
    }

    /// Access the state of the saved key immutably
    pub fn key(&self) -> &KeyPossible
    {
        &self.possible
    }

    /// Access the state of the saved key mutably
    pub fn key_mut(&mut self) -> &mut KeyPossible
    {
        &mut self.possible
    }

    /// Access the Scanner stats snapshot of when this key
    /// was saved
    pub fn stats(&self) -> &MStats
    {
        &self.stats
    }
}

/// State map tracking whether a key token is currently
/// possible in the buffer.
///
/// A key can be possible ('Yes'), impossible ('No') or
/// Required, mapping onto how confident the Scanner is that
/// a key should be added to the token queue
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::scanner) enum KeyPossible
{
    No,
    Yes,
    Required,
}

impl KeyPossible
{
    /// Is this key still possible?
    pub fn allowed(&self) -> bool
    {
        matches!(self, Self::Yes | Self::Required)
    }

    /// Is this key required?
    pub fn required(&self) -> bool
    {
        matches!(self, Self::Required)
    }
}

impl Default for KeyPossible
{
    fn default() -> Self
    {
        Self::No
    }
}
