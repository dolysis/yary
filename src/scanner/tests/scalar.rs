//! Test cases specific to scalar types. This module
//! contains three modules: plain, flow and block; further
//! fractionating the test cases into their respective
//! scalar catagories

use super::*;

mod plain
{
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn scalar_simple()
    {
        let data = "hello from a plain scalar!";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
            | Scalar(cow!("hello from a plain scalar!"), Plain)  => "expected a flow scalar (single)",
            | StreamEnd                                          => "expected end of stream",
            @ None                                               => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn scalar_starting_indicator()
    {
        let data = "-a key-: ?value\n:: :value";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | BlockMappingStart                  => "expected the start of a block mapping",
            | Key                                => "expected an explicit key",
            | Scalar(cow!("-a key-"), Plain)     => "expected a plain scalar",
            | Value                              => "expected a value",
            | Scalar(cow!("?value"), Plain)      => "expected a plain scalar",
            | Key                                => "expected an explicit key",
            | Scalar(cow!(":"), Plain)           => "expected a plain scalar",
            | Value                              => "expected a value",
            | Scalar(cow!(":value"), Plain)      => "expected a plain scalar",
            | BlockEnd                           => "expected the end of a block mapping",
            | StreamEnd                          => "expected end of stream",
            @ None                               => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }
}

mod flow
{
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn scalar_single_simple()
    {
        let data = "'hello world, single quoted flow scalar'";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)                                      => "expected start of stream",
            | Scalar(cow!("hello world, single quoted flow scalar"), SingleQuote)    => "expected a flow scalar (single)",
            | StreamEnd                                                              => "expected end of stream",
            @ None                                                                   => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn scalar_single_complex()
    {
        let data = "'line0
            line1
            
            line3
            line4'";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Scalar(cow!("line0 line1\nline3 line4"), SingleQuote)  => "expected a flow scalar (single)",
            | StreamEnd                                              => "expected end of stream",
            @ None                                                   => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn scalar_double_simple()
    {
        let data = r#""line0 line1\nline3\tline4""#;
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Scalar(cow!("line0 line1\nline3\tline4"), DoubleQuote) => "expected a flow scalar (double)",
            | StreamEnd                                              => "expected end of stream",
            @ None                                                   => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn scalar_double_complex()
    {
        let data = r#""line0
            lin\
            e1
            
            line3
            line4""#;
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Scalar(cow!("line0 line1\nline3 line4"), DoubleQuote)  => "expected a flow scalar (double)",
            | StreamEnd                                              => "expected end of stream",
            @ None                                                   => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn tag_scalar_complex()
    {
        let data = r#"
        !!str
        "line0
            lin\
            e1
            
            line3
        line4""#;
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Tag(cow!("!!"), cow!("str"))                           => "expected a secondary tag ('!!', 'str')",
            | Scalar(cow!("line0 line1\nline3 line4"), DoubleQuote)  => "expected a flow scalar (double)",
            | StreamEnd                                              => "expected end of stream",
            @ None                                                   => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }
}

mod block
{
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn scalar_literal_simple()
    {
        let data = "
a key: |  # and a comment!
    a block scalar,
    separated by new lines
";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)                                  => "expected start of stream",
            | BlockMappingStart                                                  => "expected the start of a block mapping",
            | Key                                                                => "expected an explicit key",
            | Scalar(cow!("a key"), Plain)                                       => "expected a plain scalar",
            | Value                                                              => "expected a value",
            | Scalar(cow!("a block scalar,\nseparated by new lines\n"), Literal) => "expected a block scalar (literal)",
            | BlockEnd                                                           => "expected the end of a block mapping",
            | StreamEnd                                                          => "expected end of stream",
            @ None                                                               => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn scalar_folded_simple()
    {
        let data = "
a block scalar: >-  # and a comment!
    with lines folded,
    to a space
";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | StreamStart(StreamEncoding::UTF8)                                  => "expected start of stream",
            | BlockMappingStart                                                  => "expected the start of a block mapping",
            | Key                                                                => "expected an explicit key",
            | Scalar(cow!("a block scalar"), Plain)                              => "expected a plain scalar",
            | Value                                                              => "expected a value",
            | Scalar(cow!("with lines folded, to a space"), Folded)              => "expected a block scalar (folded)",
            | BlockEnd                                                           => "expected the end of a block mapping",
            | StreamEnd                                                          => "expected end of stream",
            @ None                                                               => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }
}
