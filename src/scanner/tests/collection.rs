/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Test cases for block and flow collections specifically.
//! Note that many other token types are used in this module
//! due to the nature of collections.

use pretty_assertions::assert_eq;

use super::*;

/* === BLOCK COLLECTION TESTS === */

#[test]
fn block_sequence()
{
    let data = "
- 'a'
- 'block'
- 'sequence'
";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)      => "expected start of stream",
        | BlockSequenceStart                     => "expected a block sequence",
        | BlockEntry                             => "expected a block entry",
        | Scalar(cow!("a"), SingleQuote)         => "expected a flow scalar",
        | BlockEntry                             => "expected a block entry",
        | Scalar(cow!("block"), SingleQuote)     => "expected a flow scalar",
        | BlockEntry                             => "expected a block entry",
        | Scalar(cow!("sequence"), SingleQuote)  => "expected a flow scalar",
        | BlockEnd                               => "expected the end of a block collection",
        | StreamEnd                              => "expected end of stream",
        @ None                                   => "expected stream to be finished"
    );
}

#[test]
fn block_sequence_nested()
{
    let data = "
- - 'a'
  - 
    'nested'
- 'block'
- 'sequence'
";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)      => "expected start of stream",
        | BlockSequenceStart                     => "expected a block sequence",
        | BlockEntry                             => "expected a block entry",
        | BlockSequenceStart                     => "expected a nested block sequence",
        | BlockEntry                             => "expected a block entry",
        | Scalar(cow!("a"), SingleQuote)         => "expected a flow scalar",
        | BlockEntry                             => "expected a block entry",
        | Scalar(cow!("nested"), SingleQuote)    => "expected a flow scalar",
        | BlockEnd                               => "expected the end of the nested sequence",
        | BlockEntry                             => "expected a block entry",
        | Scalar(cow!("block"), SingleQuote)     => "expected a flow scalar",
        | BlockEntry                             => "expected a block entry",
        | Scalar(cow!("sequence"), SingleQuote)  => "expected a flow scalar",
        | BlockEnd                               => "expected the end of a block collection",
        | StreamEnd                              => "expected end of stream",
        @ None                                   => "expected stream to be finished"
    );
}

#[test]
fn block_mapping_key_only()
{
    let data = "'key': ";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | BlockMappingStart                  => "expected start of block mapping",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("key"), SingleQuote)   => "expected a flow scalar (single)",
        | Value                              => "expected a value token",
        | BlockEnd                           => "expected end of block mapping",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );
}

#[test]
fn block_mapping()
{
    let data = "'key1': 'value1'\n'key2': 'value2'";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | BlockMappingStart                  => "expected start of block mapping",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("key1"), SingleQuote)  => "expected a flow scalar (single)",
        | Value                              => "expected a value token",
        | Scalar(cow!("value1"), SingleQuote)=> "expected a flow scalar (single)",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("key2"), SingleQuote)  => "expected a flow scalar (single)",
        | Value                              => "expected a value token",
        | Scalar(cow!("value2"), SingleQuote)=> "expected a flow scalar (single)",
        | BlockEnd                           => "expected end of block mapping",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );
}

#[test]
fn block_mapping_nested()
{
    let data = "
'one':
  'key': 'value'
  'and': 'again'
";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | BlockMappingStart                  => "expected start of block mapping",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("one"), SingleQuote)   => "expected a flow scalar",
        | Value                              => "expected a value token",
        | BlockMappingStart                  => "expected start of nested mapping",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("key"), SingleQuote)   => "expected a flow scalar",
        | Value                              => "expected a value token",
        | Scalar(cow!("value"), SingleQuote) => "expected a flow scalar",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("and"), SingleQuote)   => "expected a flow scalar",
        | Value                              => "expected a value token",
        | Scalar(cow!("again"), SingleQuote) => "expected a flow scalar",
        | BlockEnd                           => "expected end of nested mapping",
        | BlockEnd                           => "expected end of block mapping",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );
}

#[test]
fn block_sequence_no_indent()
{
    let data = "
'one':
- 'two'


- 'three'
";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | BlockMappingStart                  => "expected start of block mapping",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("one"), SingleQuote)   => "expected a flow scalar",
        | Value                              => "expected a value",
        | BlockSequenceStart                 => "expected start of block sequence",
        | BlockEntry                         => "expected a sequence entry",
        | Scalar(cow!("two"), SingleQuote)   => "expected a flow scalar",
        | BlockEntry                         => "expected a sequence entry",
        | Scalar(cow!("three"), SingleQuote) => "expected a flow scalar",
        | BlockEnd                           => "expected end of nested mapping",
        | BlockEnd                           => "expected end of block mapping",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );
}

#[test]
fn block_sequence_no_indent_nested()
{
    let data = "
'one':
  'two':
  - 'three'
  'four':
  - 'five'
'six':
- 'seven'
";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | BlockMappingStart                  => "expected start of block mapping",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("one"), SingleQuote)   => "expected a flow scalar",
        | Value                              => "expected a value",
        | BlockMappingStart                  => "expected start of nested mapping",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("two"), SingleQuote)   => "expected a flow scalar",
        | Value                              => "expected a value",
        | BlockSequenceStart                 => "expected start of zero indented sequence",
        | BlockEntry                         => "expected a sequence entry",
        | Scalar(cow!("three"), SingleQuote) => "expected a flow scalar",
        | BlockEnd                           => "expected end of zero indented sequence",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("four"), SingleQuote)  => "expected a flow scalar",
        | Value                              => "expected a value",
        | BlockSequenceStart                 => "expected start of zero indented sequence",
        | BlockEntry                         => "expected a sequence entry",
        | Scalar(cow!("five"), SingleQuote)  => "expected a flow scalar",
        | BlockEnd                           => "expected end of zero indented sequence",
        | BlockEnd                           => "expected end of nested mapping",
        | Key                                => "expected an implicit key",
        | Scalar(cow!("six"), SingleQuote)   => "expected a flow scalar",
        | Value                              => "expected a value",
        | BlockSequenceStart                 => "expected start of zero indented sequence",
        | BlockEntry                         => "expected a sequence entry",
        | Scalar(cow!("seven"), SingleQuote) => "expected a flow scalar",
        | BlockEnd                           => "expected end of zero indented sequence",
        | BlockEnd                           => "expected end of block mapping",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );
}

/* === FLOW COLLECTION TESTS === */

#[test]
fn flow_mapping_indicators()
{
    let data = "{}";
    let mut s = ScanIter::new(data);

    // Note that the doubled braces here are because of Rust's
    // fmtstr escaping rules.
    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | FlowMappingStart                   => "expected a flow mapping start '{{'",
        | FlowMappingEnd                     => "expected a flow mapping end '}}'",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn flow_sequence_indicators()
{
    let data = "[]";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | FlowSequenceStart                  => "expected a flow sequence start '['",
        | FlowSequenceEnd                    => "expected a flow sequence end ']'",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn flow_mapping()
{
    let data = "{'a key': 'a value','another key': 'another value'}";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
        | FlowMappingStart                           => "expected a flow mapping start '{{'",
        | Key                                        => "expected a key",
        | Scalar(cow!("a key"), SingleQuote)         => "expected a scalar key: 'a key'",
        | Value                                      => "expected a value",
        | Scalar(cow!("a value"), SingleQuote)       => "expected a scalar value: 'a value'",
        | FlowEntry                                  => "expected a flow entry: ','",
        | Key                                        => "expected a key",
        | Scalar(cow!("another key"), SingleQuote)   => "expected a scalar key: 'another key'",
        | Value                                      => "expected a value",
        | Scalar(cow!("another value"), SingleQuote) => "expected a scalar value: 'another value'",
        | FlowMappingEnd                             => "expected a flow mapping end '}}'",
        | StreamEnd                                  => "expected end of stream",
        @ None                                       => "expected stream to be finished"
    );
}

#[test]
fn flow_sequence()
{
    let data = "['a key': 'a value','another key': 'another value']";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
        | FlowSequenceStart                          => "expected a flow sequence start '['",
        | Key                                        => "expected a key",
        | Scalar(cow!("a key"), SingleQuote)         => "expected a scalar key: 'a key'",
        | Value                                      => "expected a value",
        | Scalar(cow!("a value"), SingleQuote)       => "expected a scalar value: 'a value'",
        | FlowEntry                                  => "expected a flow entry: ','",
        | Key                                        => "expected a key",
        | Scalar(cow!("another key"), SingleQuote)   => "expected a scalar key: 'another key'",
        | Value                                      => "expected a value",
        | Scalar(cow!("another value"), SingleQuote) => "expected a scalar value: 'another value'",
        | FlowSequenceEnd                            => "expected a flow sequence end ']'",
        | StreamEnd                                  => "expected end of stream",
        @ None                                       => "expected stream to be finished"
    );
}

#[test]
fn flow_sequence_plain()
{
    let data = "[a key: a value,another key: another value]";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)      => "expected start of stream",
        | FlowSequenceStart                      => "expected a flow sequence start '['",
        | Key                                    => "expected a key",
        | Scalar(cow!("a key"), Plain)           => "expected a scalar key: 'a key'",
        | Value                                  => "expected a value",
        | Scalar(cow!("a value"), Plain)         => "expected a scalar value: 'a value'",
        | FlowEntry                              => "expected a flow entry: ','",
        | Key                                    => "expected a key",
        | Scalar(cow!("another key"), Plain)     => "expected a scalar key: 'another key'",
        | Value                                  => "expected a value",
        | Scalar(cow!("another value"), Plain)   => "expected a scalar value: 'another value'",
        | FlowSequenceEnd                        => "expected a flow sequence end ']'",
        | StreamEnd                              => "expected end of stream",
        @ None                                   => "expected stream to be finished"
    );
}

#[test]
fn flow_sequence_plain_abnormal()
{
    let data = "[-: -123,not# a comment  :  (-%!&*@`|>+-)]";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)      => "expected start of stream",
        | FlowSequenceStart                      => "expected a flow sequence start '['",
        | Key                                    => "expected a key",
        | Scalar(cow!("-"), Plain)               => "expected a scalar key: '-'",
        | Value                                  => "expected a value",
        | Scalar(cow!("-123"), Plain)            => "expected a scalar value: '-123'",
        | FlowEntry                              => "expected a flow entry: ','",
        | Key                                    => "expected a key",
        | Scalar(cow!("not# a comment"), Plain)  => "expected a scalar key: 'not# a comment'",
        | Value                                  => "expected a value",
        | Scalar(cow!("(-%!&*@`|>+-)"), Plain)   => "expected a scalar value: '(-%!&*@`|>+-)'",
        | FlowSequenceEnd                        => "expected a flow sequence end ']'",
        | StreamEnd                              => "expected end of stream",
        @ None                                   => "expected stream to be finished"
    );
}

#[test]
fn flow_nested()
{
    let data = "[
            {'a map': 'of values'},
            {'inside': 'a sequence'},
            'a string',
            ['and', 'lists','of', 'strings'],
            {'wow': {'this': {'nesting': {'goes': ['deep', '!']}}}}
        ]";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8),
        | FlowSequenceStart,
        | FlowMappingStart,
        | Key,
        | Scalar(cow!("a map"), SingleQuote),
        | Value,
        | Scalar(cow!("of values"), SingleQuote),
        | FlowMappingEnd,
        | FlowEntry,
        | FlowMappingStart,
        | Key,
        | Scalar(cow!("inside"), SingleQuote),
        | Value,
        | Scalar(cow!("a sequence"), SingleQuote),
        | FlowMappingEnd,
        | FlowEntry,
        | Scalar(cow!("a string"), SingleQuote),
        | FlowEntry,
        | FlowSequenceStart,
        | Scalar(cow!("and"), SingleQuote),
        | FlowEntry,
        | Scalar(cow!("lists"), SingleQuote),
        | FlowEntry,
        | Scalar(cow!("of"), SingleQuote),
        | FlowEntry,
        | Scalar(cow!("strings"), SingleQuote),
        | FlowSequenceEnd,
        | FlowEntry,
        | FlowMappingStart,
        | Key,
        | Scalar(cow!("wow"), SingleQuote),
        | Value,
        | FlowMappingStart,
        | Key,
        | Scalar(cow!("this"), SingleQuote),
        | Value,
        | FlowMappingStart,
        | Key,
        | Scalar(cow!("nesting"), SingleQuote),
        | Value,
        | FlowMappingStart,
        | Key,
        | Scalar(cow!("goes"), SingleQuote),
        | Value,
        | FlowSequenceStart,
        | Scalar(cow!("deep"), SingleQuote),
        | FlowEntry,
        | Scalar(cow!("!"), SingleQuote),
        | FlowSequenceEnd,
        | FlowMappingEnd,
        | FlowMappingEnd,
        | FlowMappingEnd,
        | FlowMappingEnd,
        | FlowSequenceEnd,
        | StreamEnd,
        @ None
    );

    assert_eq!(s.scan.stats, stats_of(data));
}
