//! Test cases specific to the scanning of whitespace
//! between tokens

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn empty()
{
    let data = "";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn eat()
{
    let data = "   abc";
    let mut buffer = data;
    let mut s = Scanner::new();

    s.eat_whitespace(&mut buffer, false);

    assert_eq!(buffer, "abc");
    assert_eq!(s.stats, (3, 0, 3))
}

#[test]
fn eat_none()
{
    let data = "abc";
    let mut buffer = data;
    let mut s = Scanner::new();

    s.eat_whitespace(&mut buffer, false);

    assert_eq!(buffer, "abc");
    assert_eq!(s.stats, (0, 0, 0))
}

#[test]
fn eat_comments()
{
    let data = "  # a comment\n\n#one two three\n       #four!";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}
