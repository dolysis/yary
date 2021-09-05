//! Test cases specific to document markers

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn multi_document_empty()
{
    let data = "---\n---\n---";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8),
        | DocumentStart,
        | DocumentStart,
        | DocumentStart,
        | StreamEnd,
        @ None
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn document_markers()
{
    let data = "\n---\n   \n...";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | DocumentStart                      => "expected start of document",
        | DocumentEnd                        => "expected end of document",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}

#[test]
fn comment_in_document_markers()
{
    let data = "---\n# abcdefg \n  # another comment     \n...";
    let mut s = ScanIter::new(data);

    tokens!(s =>
        | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
        | DocumentStart                      => "expected start of document",
        | DocumentEnd                        => "expected end of document",
        | StreamEnd                          => "expected end of stream",
        @ None                               => "expected stream to be finished"
    );

    assert_eq!(s.scan.stats, stats_of(data));
}
