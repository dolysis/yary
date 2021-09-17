/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Contains the structure used for tracking marker stats in
//! a buffer, namely:
//!
//! - How far into the buffer have we read?
//! - How many lines have we read?
//! - What is the current column?

use std::ops::{Add, AddAssign};

/// Vessel for tracking various stats about the underlying
/// buffer that are required for correct parsing of certain
/// elements, and when contextualizing an error.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::scanner) struct MStats
{
    /// Amount of bytes read from the underlying byte stream
    pub read:   usize,
    /// Number of lines seen in the underlying byte stream
    pub lines:  usize,
    /// The offset from the last line break into a line
    pub column: usize,
}

impl MStats
{
    /// Construct a new empty MStats instance
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Update the stored stats with the given .read .lines
    /// and .column
    pub fn update(&mut self, read: usize, lines: usize, column: usize)
    {
        self.read += read;
        self.lines += lines;

        match lines
        {
            0 => self.column += column,
            _ => self.column = column,
        }
    }
}

impl Default for MStats
{
    fn default() -> Self
    {
        Self {
            read:   0,
            lines:  0,
            column: 0,
        }
    }
}

impl Add for MStats
{
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output
    {
        self += rhs;

        self
    }
}

impl AddAssign for MStats
{
    fn add_assign(&mut self, rhs: Self)
    {
        self.update(rhs.read, rhs.lines, rhs.column)
    }
}

impl PartialEq<(usize, usize, usize)> for MStats
{
    fn eq(&self, (read, lines, column): &(usize, usize, usize)) -> bool
    {
        self.read == *read && self.lines == *lines && self.column == *column
    }
}
