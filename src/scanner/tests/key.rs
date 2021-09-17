/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Test cases specific to mapping keys, explicit or
//! implicit.

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn explicit_simple()
{
    let data = "
? 'an explicit key'
: 'a value'
";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)                         => "expected start of stream",
        | BlockMappingStart                                         => "expected the start of a block mapping",
        | Key                                                       => "expected an explicit key",
        | Scalar(cow!("an explicit key"), SingleQuote)              => "expected a scalar",
        | Value                                                     => "expected a value",
        | Scalar(cow!("a value"), SingleQuote)                      => "expected a scalar",
        | BlockEnd                                                  => "expected the end of a block mapping",
        | StreamEnd                                                 => "expected end of stream",
        @ None                                                      => "expected stream to be finished"
    );
}

#[test]
fn explicit_mapping_missing_value()
{
    // A value is implied by the explicit key, and can be
    // omitted from the document, while still being
    // valid YAML
    let data = "? 'sub mapping key': 'sub mapping value'";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)                         => "expected start of stream",
        | BlockMappingStart                                         => "expected the start of a block mapping",
        | Key                                                       => "expected an explicit key",
        | BlockMappingStart                                         => "expected the start of a block mapping",
        | Key                                                       => "expected an explicit key",
        | Scalar(cow!("sub mapping key"), SingleQuote)              => "expected a scalar",
        | Value                                                     => "expected a value",
        | Scalar(cow!("sub mapping value"), SingleQuote)            => "expected a scalar",
        | BlockEnd                                                  => "expected the end of a block mapping",
        | BlockEnd                                                  => "expected the end of a block mapping",
        | StreamEnd                                                 => "expected end of stream",
        @ None                                                      => "expected stream to be finished"
    );
}

#[test]
fn explicit_mapping()
{
    let data = "
? 'key mapping': 'value'
  'another': 'value'
: 'bar'
";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)                         => "expected start of stream",
        | BlockMappingStart                                         => "expected the start of a block mapping",
        | Key                                                       => "expected an explicit key",
        | BlockMappingStart                                         => "expected the start of a block mapping",
        | Key                                                       => "expected an explicit key",
        | Scalar(cow!("key mapping"), SingleQuote)                  => "expected a scalar",
        | Value                                                     => "expected a value",
        | Scalar(cow!("value"), SingleQuote)                        => "expected a scalar",
        | Key                                                       => "expected an explicit key",
        | Scalar(cow!("another"), SingleQuote)                      => "expected a scalar",
        | Value                                                     => "expected a value",
        | Scalar(cow!("value"), SingleQuote)                        => "expected a scalar",
        | BlockEnd                                                  => "expected the end of a block mapping",
        | Value                                                     => "expected a value",
        | Scalar(cow!("bar"), SingleQuote)                          => "expected a scalar",
        | BlockEnd                                                  => "expected the end of a block mapping",
        | StreamEnd                                                 => "expected end of stream",
        @ None                                                      => "expected stream to be finished"
    );
}

#[test]
fn stale_required_oversized()
{
    let expiry_len = std::str::from_utf8(&[b' '; 1025]).unwrap();
    let data = format!(
        "
'a map': 'a key'
'key start... then SPACE!!! {}': 'a value'",
        expiry_len
    );

    let mut s = ScanIter::new(&data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8),
        | BlockMappingStart,
        | Key,
        | Scalar(cow!("a map"), SingleQuote),
        | Value,
        | Scalar(cow!("a key"), SingleQuote),
        > Result::<Token>::Err(ScanError::MissingValue) => "expected an error due to a required key"
    );
}

#[test]
fn stale_oversized()
{
    let expiry_len = std::str::from_utf8(&[b' '; 1025]).unwrap();
    let data = format!(
        "
'key start... then SPACE!!! {}': 'a value'",
        expiry_len
    );

    let mut s = ScanIter::new(&data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8),
        > Result::<Token>::Err(ScanError::InvalidValue) => "expected an error due to oversized key"
    );
}

#[test]
fn stale_after_newline()
{
    let data = "
'a multi-line
 key': 'a value'
";

    let mut s = ScanIter::new(&data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8),
        > Result::<Token>::Err(ScanError::InvalidValue) => "expected an error due to multi-line key"
    );
}
