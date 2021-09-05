//! Test cases specific to directives

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn directive_version()
{
    let data = "%YAML   1.1 # a comment\n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | VersionDirective(1, 1)             => "expected version directive (1, 1)",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn directive_version_large()
{
    let data = "%YAML   121.80 # a comment\n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | VersionDirective(121, 80)          => "expected version directive (121, 80)",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn directive_version_invalid()
{
    let data = "%YAML   foo.bar # a comment\n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)                 => "expected start of stream",
        > Result::<Token>::Err(ScanError::InvalidVersion)   => "expected an version directive error"
    );

    assert_eq!(s.scan.stats, stats_of(&data[0..0]));
}

#[test]
fn directive_tag_named()
{
    let data = "%TAG !named! my:cool:tag # a comment\n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
        | TagDirective(cow!("!named!"), cow!("my:cool:tag")) => "expected named tag directive",
        | StreamEnd                                          => "expected end of stream",
        @ None                                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn directive_tag_primary()
{
    let data = "%TAG ! my:cool:tag\n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)              => "expected start of stream",
        | TagDirective(cow!("!"), cow!("my:cool:tag"))   => "expected primary tag directive",
        | StreamEnd                                      => "expected end of stream",
        @ None                                           => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn directive_tag_secondary()
{
    let data = "%TAG !! @my/crazy&tag:  \n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
        | TagDirective(cow!("!!"), cow!("@my/crazy&tag:"))   => "expected secondary tag directive",
        | StreamEnd                                          => "expected end of stream",
        @ None                                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn directive_tag_ending_ws()
{
    let data = "%TAG !! @my/crazy&tag:";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
        > Result::<Token>::Err(ScanError::UnexpectedEOF)    => "expected an eof error"
    );

    assert_eq!(s.scan.stats, stats_of(&data[0..0]));
}

#[test]
fn directive_tag_percent_encoding()
{
    let data = "%TAG !! :My:%C6%86razy:T%c8%82g:\n";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
        | TagDirective(cow!("!!"), cow!(":My:Ɔrazy:TȂg:"))   => "expected unescaped unicode prefix",
        | StreamEnd                                          => "expected end of stream",
        @ None                                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}
