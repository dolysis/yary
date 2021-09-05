//! Test cases specific to node tags

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn primary()
{
    let data = "!a ";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)     => "expected start of stream",
        | Tag(cow!("!"), cow!("a"))             => "expected a primary tag ('!', 'a')",
        | StreamEnd                             => "expected end of stream",
        @ None                                  => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn secondary()
{
    let data = "!!str ";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | Tag(cow!("!!"), cow!("str"))       => "expected a secondary tag ('!!', 'str')",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn named()
{
    let data = "    !named!tag-suffix ";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)         => "expected start of stream",
        | Tag(cow!("!named!"), cow!("tag-suffix"))  => "expected a global tag ('!named!', 'tag-suffix')",
        | StreamEnd                                 => "expected end of stream",
        @ None                                      => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn escaped()
{
    let data = "!n!my:%3D%3descaped: ";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)         => "expected start of stream",
        | Tag(cow!("!n!"), cow!("my:==escaped:"))   => "expected a global tag ('!n!', 'my:==escaped:')",
        | StreamEnd                                 => "expected end of stream",
        @ None                                      => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn non_resolving()
{
    let data = "! ";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
        | Tag(cow!("!"), cow!(""))                   => "expected a non resolving tag ('!', '')",
        | StreamEnd                                  => "expected end of stream",
        @ None                                       => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, (2, 0, 2));
}
