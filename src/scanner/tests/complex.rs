/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Test cases that contain any combination of valid YAML
//! tokens. Typically any tests that are closer to YAML
//! documents rather than specific fragments belong here, or
//! tests for the interaction between multiple tokens -- and
//! Scanner subsystems.

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn no_map_sequence_scalar()
{
    let data = r##"

---

%YAML           1.2                     # our document's version.
%TAG !          primary:namespace       # our doc's primary tag
%TAG !!         secondary/namespace:    # our doc's secondary tag
%TAG !named0!   named0:                 # A named tag

&ref
*ref



...

"##;
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8),
        | DocumentStart,
        | VersionDirective(1, 2),
        | TagDirective(cow!("!"), cow!("primary:namespace")),
        | TagDirective(cow!("!!"), cow!("secondary/namespace:")),
        | TagDirective(cow!("!named0!"), cow!("named0:")),
        | Anchor(cow!("ref")),
        | Alias(cow!("ref")),
        | DocumentEnd,
        | StreamEnd,
        @ None
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn no_map_sequence()
{
    let data = r##"

%YAML           1.2                     # our document's version.
%TAG !          primary:namespace       # our doc's primary tag
%TAG !!         secondary/namespace:    # our doc's secondary tag
%TAG !named0!   named0:                 # A named tag
---

!!str "an anchor": &ref !value 'some   
                                value'
!!str 'an alias': *ref

...

"##;
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8),
        | VersionDirective(1, 2),
        | TagDirective(cow!("!"), cow!("primary:namespace")),
        | TagDirective(cow!("!!"), cow!("secondary/namespace:")),
        | TagDirective(cow!("!named0!"), cow!("named0:")),
        | DocumentStart,
        | BlockMappingStart,
        | Key,
        | Tag(cow!("!!"), cow!("str")),
        | Scalar(cow!("an anchor"), DoubleQuote),
        | Value,
        | Anchor(cow!("ref")),
        | Tag(cow!("!"), cow!("value")),
        | Scalar(cow!("some value"), SingleQuote),
        | Key,
        | Tag(cow!("!!"), cow!("str")),
        | Scalar(cow!("an alias"), SingleQuote),
        | Value,
        | Alias(cow!("ref")),
        | BlockEnd,
        | DocumentEnd,
        | StreamEnd,
        @ None
    );
}

#[test]
fn plain()
{
    let data = r##"

---
- [
    key: value,
        indented: value,
        {an object: inside a sequence},
        [sequence inception!]
]
-   lets do it: &val as block,
    can we :
        build it:
            higher?: *val
    yes: we
    can: baby

                    "##;

    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8),
        | DocumentStart,
        | BlockSequenceStart,
        | BlockEntry,
        | FlowSequenceStart,
        | Key,
        | Scalar(cow!("key"), Plain),
        | Value,
        | Scalar(cow!("value"), Plain),
        | FlowEntry,
        | Key,
        | Scalar(cow!("indented"), Plain),
        | Value,
        | Scalar(cow!("value"), Plain),
        | FlowEntry,
        | FlowMappingStart,
        | Key,
        | Scalar(cow!("an object"), Plain),
        | Value,
        | Scalar(cow!("inside a sequence"), Plain),
        | FlowMappingEnd,
        | FlowEntry,
        | FlowSequenceStart,
        | Scalar(cow!("sequence inception!"), Plain),
        | FlowSequenceEnd,
        | FlowSequenceEnd,
        | BlockEntry,
        | BlockMappingStart,
        | Key,
        | Scalar(cow!("lets do it"), Plain),
        | Value,
        | Anchor(cow!("val")),
        | Scalar(cow!("as block,"), Plain),
        | Key,
        | Scalar(cow!("can we"), Plain),
        | Value,
        | BlockMappingStart,
        | Key,
        | Scalar(cow!("build it"), Plain),
        | Value,
        | BlockMappingStart,
        | Key,
        | Scalar(cow!("higher?"), Plain),
        | Value,
        | Alias(cow!("val")),
        | BlockEnd,
        | BlockEnd,
        | Key,
        | Scalar(cow!("yes"), Plain),
        | Value,
        | Scalar(cow!("we"), Plain),
        | Key,
        | Scalar(cow!("can"), Plain),
        | Value,
        | Scalar(cow!("baby"), Plain),
        | BlockEnd,
        | BlockEnd,
        | StreamEnd,
        @ None
    );
}
