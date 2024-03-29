/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Test cases specific to node anchors and aliases.

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn anchor_alias()
{
    let data = "*alias\n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)     => "expected start of stream",
        | Alias(cow!("alias"))                  => "expected an alias named 'alias'",
        | StreamEnd                             => "expected end of stream",
        @ None                                  => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn anchor()
{
    let data = "    &anchor     \n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)     => "expected start of stream",
        | Anchor(cow!("anchor"))                => "expected an anchor named 'anchor'",
        | StreamEnd                             => "expected end of stream",
        @ None                                  => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}
