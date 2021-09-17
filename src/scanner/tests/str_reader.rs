/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use cfg_if::cfg_if;

#[derive(Debug, Clone)]
pub(super) struct StrReader<'de>
{
    s:    &'de str,
    size: usize,
}

impl<'de> StrReader<'de>
{
    cfg_if! {
        if #[cfg(feature = "test_buffer_large")]
        {
            pub const BUF_SIZE: usize = 4 * 1024;
            pub const BUF_EXTEND: usize = 64;
        }
        else if #[cfg(feature = "test_buffer_medium")]
        {
            pub const BUF_SIZE: usize = 8;
            pub const BUF_EXTEND: usize = 8;
        }
        else if #[cfg(feature = "test_buffer_small")]
        {
            pub const BUF_SIZE: usize = 1;
            pub const BUF_EXTEND: usize = 1;
        }
    }

    pub fn new(s: &'de str, size: usize) -> Self
    {
        let size = std::cmp::min(s.len(), size);

        Self { s, size }
    }

    pub fn read(&self) -> &'de str
    {
        &self.s[..self.size]
    }

    pub fn expand(&mut self, size: usize)
    {
        let new = self.size + size;

        match self.s.len() > new
        {
            true => self.size = new,
            false => self.size = self.s.len(),
        }
    }

    pub fn expandable(&self) -> bool
    {
        self.size < self.s.len()
    }
}

impl std::fmt::Display for StrReader<'_>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        self.s.fmt(f)
    }
}
