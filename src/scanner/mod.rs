// Note that this module must come before all others, as
// they depend on the macros which expand into this scope
#[macro_use]
mod macros;

mod error;
mod key;
mod scalar;
mod tag;

use std::ops::{Add, AddAssign};

use atoi::atoi;

use self::{
    error::{ScanError, ScanResult as Result},
    key::Key,
};
use crate::{
    scanner::{
        scalar::flow::scan_flow_scalar,
        tag::{scan_node_tag, scan_tag_directive},
    },
    token::{Ref, StreamEncoding, Token},
};

#[derive(Debug)]
struct Scanner<'b>
{
    buffer: &'b str,
    stats:  MStats,
    state:  StreamState,
    key:    Key,
}

impl<'b> Scanner<'b>
{
    pub fn new(data: &'b str) -> Self
    {
        Self {
            buffer: data,
            stats:  MStats::new(),
            state:  StreamState::Start,
            key:    Key::default(),
        }
    }

    fn next_token<'c>(&mut self, scratch: &'c mut Vec<u8>) -> Result<Option<Ref<'b, 'c>>>
    {
        if self.key.has_tokens()
        {
            match self.key.next_token(self.buffer, scratch)?
            {
                Some((token, amt)) =>
                {
                    advance!(self.buffer, :self.stats, amt);

                    Ok(Some(token))
                },
                _ => unreachable!("{}: while parsing a key sequence", BUG),
            }
        }
        else
        {
            self.scan_next_token(scratch)
        }
    }

    fn scan_next_token<'c>(&mut self, scratch: &'c mut Vec<u8>) -> Result<Option<Ref<'b, 'c>>>
    {
        self.reset_stale_key();

        if let Some(begin) = self.start_stream()
        {
            return Ok(begin.borrowed().into());
        }

        self.eat_whitespace(COMMENTS);

        if let Some(end) = self.stream_end()
        {
            return Ok(end.borrowed().into());
        }

        if let Some(document) = self.document_marker()
        {
            return Ok(document.borrowed().into());
        }

        /*
         * The borrow checker currently does not understand that
         * each "if let ..." above is terminal, and complains
         * that possible multi mutable borrows of .scratch can
         * occur. Its wrong, but I can't convince it the
         * pattern is safe, so the best I can do is to lift the
         * actual checks each function performs up into a
         * match statement that the compiler understands
         * is terminal.
         *
         * Hopefully in future this issue will be resolved and I
         * can remove this match in favour of if guards
         */
        match self.buffer.as_bytes()
        {
            [DIRECTIVE, ..] => match self.directive(scratch)?
            {
                directive @ Some(_) => Ok(directive),
                None => unreachable!("{}: while scanning a directive", BUG),
            },
            [ANCHOR, ..] | [ALIAS, ..] => match self.anchor()?
            {
                Some(token) => Ok(Some(token.borrowed())),
                None => unreachable!("{}: while scanning an anchor", BUG),
            },
            [TAG, ..] => match self.tag(scratch)?
            {
                tag @ Some(_) => Ok(tag),
                None => unreachable!("{}: while scanning a tag", BUG),
            },
            [SINGLE, ..] | [DOUBLE, ..] => match self.flow_scalar(scratch)?
            {
                fscalar @ Some(_) => Ok(fscalar),
                None => unreachable!("{}: while parsing a flow scalar", BUG),
            },
            [VALUE, ..] if isWhiteSpaceZ!(~self.buffer, 1) => match self.value()?
            {
                value @ Some(_) => Ok(value),
                None => unreachable!("{}: while parsing a value", BUG),
            },
            _ => Ok(None),
        }
    }

    fn start_stream(&mut self) -> Option<Token<'b>>
    {
        match self.state
        {
            StreamState::Start =>
            {
                // A key is allowed at the beginning of the stream
                self.key.possible(!REQUIRED);

                self.state = StreamState::Stream;

                Some(Token::StreamStart(StreamEncoding::UTF8))
            },
            _ => None,
        }
    }

    fn stream_end(&mut self) -> Option<Token<'b>>
    {
        match (self.state, self.buffer.is_empty())
        {
            (StreamState::Done, _) => None,
            (_, true) =>
            {
                self.state = StreamState::Done;

                Some(Token::StreamEnd)
            },
            (_, false) => None,
        }
    }

    /// Chomp whitespace and optionally comments until we
    /// reach the next token, updating buffer[0] to the
    /// beginning of the new token
    fn eat_whitespace(&mut self, comments: bool) -> usize
    {
        let mut stats = MStats::new();

        let amt = eat_whitespace(&self.buffer, &mut stats, comments);

        // A new line may start a key in the block context
        //
        // FIXME: we don't track flow/block contexts yet, add check
        // here when we do
        if stats.lines != 0
        {
            self.key.possible(!REQUIRED);
        }

        advance!(self.buffer, amt);
        self.stats += stats;

        amt
    }

    fn document_marker(&mut self) -> Option<Token<'b>>
    {
        if self.stats.column == 0 && isWhiteSpaceZ!(~self.buffer, 3)
        {
            let token = match self.buffer.as_bytes()
            {
                [b'-', b'-', b'-', ..] => Token::DocumentStart,
                [b'.', b'.', b'.', ..] => Token::DocumentEnd,
                _ => return None,
            };

            // A key cannot follow a document marker
            // (though a scalar can)
            self.key.impossible();

            advance!(self.buffer, :self.stats, 3);

            return Some(token);
        }

        None
    }

    fn directive<'c>(&mut self, scratch: &'c mut Vec<u8>) -> Result<Option<Ref<'b, 'c>>>
    {
        let mut buffer = self.buffer;
        let mut stats = MStats::new();

        if !check!(~buffer => [DIRECTIVE, ..])
        {
            return Ok(None);
        }

        // Safety: we check above that we have len >= 1 (e.g a '%')
        //
        // %YAML 1.1
        //  ^^^^
        // %TAG
        //  ^^^
        let kind = DirectiveKind::new(&buffer[1..])?;

        // '%' + 'YAML' or 'TAG'
        advance!(buffer, :stats, 1 + kind.len());

        let token = match kind
        {
            DirectiveKind::Version =>
            {
                // Chomp any preceding whitespace
                advance!(buffer, eat_whitespace(buffer, &mut stats, !COMMENTS));

                // %YAML 1.1
                //       ^
                let (major, skip) = scan_directive_version(buffer)?;
                advance!(buffer, :stats, skip);

                // %YAML 1.1
                //        ^
                check!(~buffer => b'.', else ScanError::InvalidVersion)?;
                advance!(buffer, :stats, 1);

                // %YAML 1.1
                //         ^
                let (minor, skip) = scan_directive_version(buffer)?;
                advance!(buffer, :stats, skip);

                Token::VersionDirective(major, minor).borrowed()
            },
            DirectiveKind::Tag =>
            {
                // Chomp any spaces up to the handle
                advance!(buffer, eat_whitespace(buffer, &mut stats, !COMMENTS));

                // Scan the directive, copying if necessary
                let (token, amt) = scan_tag_directive(buffer, &mut stats, scratch)?;
                advance!(buffer, amt);

                token
            },
        };

        // A key cannot follow a directive
        self.key.impossible();

        // %YAML 1.1 # some comment\n
        //          ^^^^^^^^^^^^^^^^^ buffer
        // ^^^^^^^^^ self.buffer.len - buffer.len
        advance!(self.buffer, self.buffer.len() - buffer.len());
        self.stats += stats;

        Ok(Some(token))
    }

    /// Try eat a tag, returning a Token if one could be
    /// found at the current buffer head, or none if one
    /// couldn't.
    fn tag<'c>(&mut self, scratch: &'c mut Vec<u8>) -> Result<Option<Ref<'b, 'c>>>
    {
        let mut buffer = self.buffer;
        let mut stats = MStats::new();

        if !check!(~buffer => [TAG, ..])
        {
            return Ok(None);
        }

        let (token, amt) = scan_node_tag(buffer, &mut stats, scratch)?;
        advance!(buffer, amt);

        // A key is possible after a tag
        self.key.possible(!REQUIRED);

        // !named_tag!type-suffix "my tagged value"
        //                       ^^^^^^^^^^^^^^^^^^ buffer
        // ^^^^^^^^^^^^^^^^^^^^^^ self.buffer.len - buffer.len
        advance!(self.buffer, self.buffer.len() - buffer.len());
        self.stats += stats;

        Ok(Some(token))
    }

    fn anchor(&mut self) -> Result<Option<Token<'b>>>
    {
        let mut buffer = self.buffer;
        let mut stats = MStats::new();

        // *anchor 'rest of the line'
        // ^
        let kind = match buffer.as_bytes()
        {
            [b @ ALIAS, ..] | [b @ ANCHOR, ..] =>
            {
                AnchorKind::new(b).expect("we only bind * or & so this cannot fail")
            },
            _ => return Ok(None),
        };

        advance!(buffer, :stats, 1);

        // *anchor 'rest of the line'
        //  ^^^^^^
        let anchor = take_while(buffer.as_bytes(), u8::is_ascii_alphanumeric);

        let anchor = advance!(<- buffer, :stats, anchor.len());

        // anchor name cannot be empty, must contain >= 1
        // alphanumeric character
        if anchor.is_empty()
        {
            return Err(ScanError::InvalidAnchorName);
        }

        // *anchor 'rest of the line'
        //        ^
        // There does not necessarily need to be a whitespace so we
        // also check against a list of valid starting
        // tokens

        check!(~buffer
            => b' ' | b'\n' | b'?' | b',' | b']' | b'}' | b'%' | b'@' | b'`',
            else ScanError::InvalidAnchorName
        )?;

        let token = match kind
        {
            AnchorKind::Alias => Token::Alias(cow!(anchor)),
            AnchorKind::Anchor => Token::Anchor(cow!(anchor)),
        };

        // A key is possible after an anchor or alias
        self.key.possible(!REQUIRED);

        // *anchor 'rest of the line'
        //        ^^^^^^^^^^^^^^^^^^^ buffer.len
        // ^^^^^^^ self.buffer.len - buffer.len
        advance!(self.buffer, self.buffer.len() - buffer.len());
        self.stats += stats;

        Ok(Some(token))
    }

    fn flow_scalar<'c>(&mut self, scratch: &'c mut Vec<u8>) -> Result<Option<Ref<'b, 'c>>>
    {
        let mut buffer = self.buffer;
        let mut stats = MStats::new();
        let single = check!(~buffer => [SINGLE, ..]);

        if !check!(~buffer => [SINGLE, ..] | [DOUBLE, ..])
        {
            return Ok(None);
        }

        let (range, amt) = scan_flow_scalar(buffer, &mut stats, scratch, single)?;

        if self.key.allowed() && check_is_key(&buffer[amt..])
        {
            self.key.impossible();

            self.key.save(range, amt);

            let (token, amt) = self.key.next_token(buffer, scratch)?.unwrap();
            advance!(self.buffer, :self.stats, amt);

            return Ok(Some(token));
        }

        let token = range.into_token(buffer, scratch)?;
        advance!(buffer, amt);

        // A key cannot follow a flow scalar, as we're either
        // currently in a key (which should be followed by a
        // value), or a value which needs a separator before
        // another key is legal
        self.key.impossible();

        advance!(self.buffer, self.buffer.len() - buffer.len());
        self.stats += stats;

        Ok(Some(token))
    }

    fn value(&mut self) -> Result<Option<Ref<'b, 'static>>>
    {
        let mut buffer = self.buffer;
        let mut stats = MStats::new();

        if !(check!(~buffer => [VALUE, ..]) && isWhiteSpaceZ!(~buffer, 1))
        {
            return Ok(None);
        }

        let token = Token::Value.borrowed();
        advance!(buffer, :stats, 1);

        // A key cannot follow a value
        self.key.impossible();

        advance!(self.buffer, self.buffer.len() - buffer.len());
        self.stats += stats;

        Ok(Some(token))
    }

    fn reset_stale_key(&mut self)
    {
        /*
         * The YAML spec requires that an implicit key cannot
         * exceed 1024 characters, and must be contained to
         * a single line
         *
         * https://yaml.org/spec/1.2/spec.html#ns-s-implicit-yaml-key(c)
         *
         * P.S: scroll up 10 lines
         */

        let exceeded_max_len = self
            .stats
            .read
            .checked_sub(self.key.mark)
            .filter(|diff| *diff > 1024)
            .is_some();

        if exceeded_max_len || self.key.line != self.stats.lines
        {
            self.key = Key::new(self.stats.read, self.stats.lines)
        }
    }
}

struct ScanIter<'a, 'b, 'c>
{
    inner:   &'a mut Scanner<'b>,
    scratch: &'c mut Vec<u8>,
}

impl<'a, 'b, 'c> ScanIter<'a, 'b, 'c>
{
    pub fn new(inner: &'a mut Scanner<'b>, scratch: &'c mut Vec<u8>) -> Self
    {
        Self { inner, scratch }
    }

    pub fn next(&mut self) -> Option<Result<Ref<'_, '_>>>
    {
        dbg!(self.inner.next_token(self.scratch).transpose())
    }
}

enum DirectiveKind
{
    Version,
    Tag,
}

impl DirectiveKind
{
    const V_LEN: usize = 4;
    const T_LEN: usize = 3;

    fn new(b: &str) -> Result<Self>
    {
        if b.starts_with("YAML")
        {
            Ok(Self::Version)
        }
        else if b.starts_with("TAG")
        {
            Ok(Self::Tag)
        }
        else
        {
            Err(ScanError::UnknownDirective)
        }
    }

    fn len(&self) -> usize
    {
        match self
        {
            Self::Version => Self::V_LEN,
            Self::Tag => Self::T_LEN,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum AnchorKind
{
    Anchor,
    Alias,
}

impl AnchorKind
{
    pub fn new(b: &u8) -> Option<Self>
    {
        let s = match b
        {
            b'*' => Self::Alias,
            b'&' => Self::Anchor,
            _ => return None,
        };

        Some(s)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum StreamState
{
    Start,
    Stream,
    Done,
}

fn scan_directive_version(b: &str) -> Result<(u8, usize)>
{
    let v_slice = take_while(b.as_bytes(), u8::is_ascii_digit);
    let v = atoi(v_slice).ok_or(ScanError::InvalidVersion)?;

    Ok((v, v_slice.len()))
}

fn take_while<F>(b: &[u8], f: F) -> &[u8]
where
    F: Fn(&u8) -> bool,
{
    let mut index = 0;

    loop
    {
        match b.get(index)
        {
            Some(b) if f(b) => index += 1,
            _ => return &b[..index],
        }
    }
}

/// Chomp whitespace and .comments if allowed until a non
/// whitespace character is encountered, returning the
/// amount chomped
fn eat_whitespace(base: &str, stats: &mut MStats, comments: bool) -> usize
{
    let mut buffer = base;
    let mut chomp_line = false;
    let mut done = false;

    loop
    {
        let (blank, brk) = (isBlank!(~buffer), isBreak!(~buffer));

        match (blank, brk)
        {
            // Non break whitespace
            (true, _) =>
            {},
            // Break whitespace, reset .chomp_line if set
            (_, true) => chomp_line = false,
            // If we're allowed to eat .comments, chomp the whole line
            _ if comments && check!(~buffer => b'#') => chomp_line = true,
            // Eat everything until EOL or EOF
            _ if chomp_line && !check!(~buffer => []) =>
            {},
            // Otherwise we're finished, exit the loop
            _ => done = true,
        }

        if done
        {
            break;
        }

        if brk
        {
            advance!(buffer, :stats, @line);
        }
        else
        {
            advance!(buffer, :stats, 1);
        }
    }

    base.len() - buffer.len()
}

fn check_is_key(buffer: &str) -> bool
{
    let amt = eat_whitespace(buffer, &mut MStats::new(), !COMMENTS);

    // FIXME: we need to support io::Read based buffers,
    // and need some way to hint that the buffer should
    // contain <N> more bytes, fetching them if required
    check!(~buffer, amt => [VALUE, ..]) && isWhiteSpaceZ!(~buffer, 1)
}

/// Vessel for tracking various stats about the underlying
/// buffer that are required for correct parsing of certain
/// elements, and when contextualizing an error.
#[derive(Debug, Clone, PartialEq)]
struct MStats
{
    read:   usize,
    lines:  usize,
    column: usize,
}

impl MStats
{
    fn new() -> Self
    {
        Self::default()
    }

    fn update(&mut self, read: usize, lines: usize, column: usize)
    {
        self.read += read;
        self.lines += lines;

        match lines
        {
            0 => self.column += column,
            _ => self.column = column,
        }
    }
}

impl Default for MStats
{
    fn default() -> Self
    {
        Self {
            read:   0,
            lines:  0,
            column: 0,
        }
    }
}

impl Add for MStats
{
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output
    {
        self += rhs;

        self
    }
}

impl AddAssign for MStats
{
    fn add_assign(&mut self, rhs: Self)
    {
        self.update(rhs.read, rhs.lines, rhs.column)
    }
}

impl PartialEq<(usize, usize, usize)> for MStats
{
    fn eq(&self, (read, lines, column): &(usize, usize, usize)) -> bool
    {
        self.read == *read && self.lines == *lines && self.column == *column
    }
}

const DIRECTIVE: u8 = b'%';
const ANCHOR: u8 = b'&';
const ALIAS: u8 = b'*';
const TAG: u8 = b'!';
const SINGLE: u8 = b'\'';
const DOUBLE: u8 = b'"';
const VALUE: u8 = b':';

const COMMENTS: bool = true;
const REQUIRED: bool = true;
const BUG: &str = "LIBRARY BUG!! HIT AN UNREACHABLE STATEMENT";

#[cfg(test)]
mod tests
{
    #[macro_use]
    mod macros;

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::token::ScalarStyle;

    #[test]
    fn empty()
    {
        let data = "";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn multi_document_empty()
    {
        let data = "---\n---\n---";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8),
            | Token::DocumentStart,
            | Token::DocumentStart,
            | Token::DocumentStart,
            | Token::StreamEnd,
            @ None
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn document_markers()
    {
        let data = "\n---\n   \n...";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::DocumentStart                      => "expected start of document",
            | Token::DocumentEnd                        => "expected end of document",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn chomp_comments()
    {
        let data = "  # a comment\n\n#one two three\n       #four!";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn comment_in_document_markers()
    {
        let data = "---\n# abcdefg \n  # another comment     \n...";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::DocumentStart                      => "expected start of document",
            | Token::DocumentEnd                        => "expected end of document",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn directive_version()
    {
        let data = "%YAML   1.1 # a comment\n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::VersionDirective(1, 1)             => "expected version directive (1, 1)",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn directive_version_large()
    {
        let data = "%YAML   121.80 # a comment\n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::VersionDirective(121, 80)          => "expected version directive (121, 80)",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn directive_version_invalid()
    {
        let data = "%YAML   foo.bar # a comment\n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            > Result::<Token>::Err(ScanError::InvalidVersion)   => "expected an version directive error"
        );

        assert_eq!(s.stats, stats_of(&data[0..0]));
    }

    #[test]
    fn directive_tag_named()
    {
        let data = "%TAG !named! my:cool:tag # a comment\n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
            | Token::TagDirective(cow!("!named!"), cow!("my:cool:tag")) => "expected named tag directive",
            | Token::StreamEnd                                          => "expected end of stream",
            @ None                                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn directive_tag_primary()
    {
        let data = "%TAG ! my:cool:tag\n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)              => "expected start of stream",
            | Token::TagDirective(cow!("!"), cow!("my:cool:tag"))   => "expected primary tag directive",
            | Token::StreamEnd                                      => "expected end of stream",
            @ None                                                  => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn directive_tag_secondary()
    {
        let data = "%TAG !! @my/crazy&tag:  \n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
            | Token::TagDirective(cow!("!!"), cow!("@my/crazy&tag:"))   => "expected secondary tag directive",
            | Token::StreamEnd                                          => "expected end of stream",
            @ None                                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn directive_tag_ending_ws()
    {
        let data = "%TAG !! @my/crazy&tag:";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            > Result::<Token>::Err(ScanError::UnexpectedEOF)    => "expected an eof error"
        );

        assert_eq!(s.stats, stats_of(&data[0..0]));
    }

    #[test]
    fn directive_tag_percent_encoding()
    {
        let data = "%TAG !! :My:%C6%86razy:T%c8%82g:\n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
            | Token::TagDirective(cow!("!!"), cow!(":My:Ɔrazy:TȂg:"))   => "expected unescaped unicode prefix",
            | Token::StreamEnd                                          => "expected end of stream",
            @ None                                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn anchor_alias()
    {
        let data = "*alias\n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Alias(cow!("alias"))               => "expected an alias named 'alias'",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn anchor()
    {
        let data = "    &anchor     \n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Anchor(cow!("anchor"))             => "expected an anchor named 'anchor'",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn tag_primary()
    {
        let data = "!a ";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Tag(cow!("!"), cow!("a"))          => "expected a primary tag ('!', 'a')",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn tag_secondary()
    {
        let data = "!!str ";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Tag(cow!("!!"), cow!("str"))       => "expected a secondary tag ('!!', 'str')",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn tag_named()
    {
        let data = "    !named!tag-suffix ";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            | Token::Tag(cow!("!named!"), cow!("tag-suffix"))   => "expected a global tag ('!named!', 'tag-suffix')",
            | Token::StreamEnd                                  => "expected end of stream",
            @ None                                              => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn tag_escaped()
    {
        let data = "!n!my:%3D%3descaped: ";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            | Token::Tag(cow!("!n!"), cow!("my:==escaped:"))    => "expected a global tag ('!n!', 'my:==escaped:')",
            | Token::StreamEnd                                  => "expected end of stream",
            @ None                                              => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn tag_non_resolving()
    {
        let data = "! ";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            | Token::Tag(cow!("!"), cow!(""))                   => "expected a non resolving tag ('!', '')",
            | Token::StreamEnd                                  => "expected end of stream",
            @ None                                              => "expected stream to be finished"
        );

        assert_eq!(s.stats, (2, 0, 2));
    }

    #[test]
    fn flow_scalar_single_simple()
    {
        use ScalarStyle::SingleQuote;

        let data = "'hello world, single quoted flow scalar'";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                                      => "expected start of stream",
            | Token::Scalar(cow!("hello world, single quoted flow scalar"), SingleQuote)    => "expected a flow scalar (single)",
            | Token::StreamEnd                                                              => "expected end of stream",
            @ None                                                                          => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn flow_scalar_single_complex()
    {
        use ScalarStyle::SingleQuote;

        let data = "'line0
            line1
            
            line3
            line4'";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Token::Scalar(cow!("line0 line1\nline3 line4"), SingleQuote)  => "expected a flow scalar (single)",
            | Token::StreamEnd                                              => "expected end of stream",
            @ None                                                          => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn flow_scalar_double_simple()
    {
        use ScalarStyle::DoubleQuote;

        let data = r#""line0 line1\nline3\tline4""#;
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Token::Scalar(cow!("line0 line1\nline3\tline4"), DoubleQuote) => "expected a flow scalar (double)",
            | Token::StreamEnd                                              => "expected end of stream",
            @ None                                                          => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn flow_scalar_double_complex()
    {
        use ScalarStyle::DoubleQuote;

        let data = r#""line0
            lin\
            e1
            
            line3
            line4""#;
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Token::Scalar(cow!("line0 line1\nline3 line4"), DoubleQuote)  => "expected a flow scalar (double)",
            | Token::StreamEnd                                              => "expected end of stream",
            @ None                                                          => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn tag_flow_scalar_complex()
    {
        use ScalarStyle::DoubleQuote;

        let data = r#"
        !!str
        "line0
            lin\
            e1
            
            line3
        line4""#;
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Token::Tag(cow!("!!"), cow!("str"))                           => "expected a secondary tag ('!!', 'str')",
            | Token::Scalar(cow!("line0 line1\nline3 line4"), DoubleQuote)  => "expected a flow scalar (double)",
            | Token::StreamEnd                                              => "expected end of stream",
            @ None                                                          => "expected stream to be finished"
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn implicit_key_simple()
    {
        use ScalarStyle::SingleQuote;

        let data = "'key': ";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Key                                => "expected an implicit key",
            | Token::Scalar(cow!("key"), SingleQuote)   => "expected a flow scalar (single)",
            | Token::Value                              => "expected a value token",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );
    }

    #[test]
    fn complex_no_map_sequence_scalar()
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
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8),
            | Token::DocumentStart,
            | Token::VersionDirective(1, 2),
            | Token::TagDirective(cow!("!"), cow!("primary:namespace")),
            | Token::TagDirective(cow!("!!"), cow!("secondary/namespace:")),
            | Token::TagDirective(cow!("!named0!"), cow!("named0:")),
            | Token::Anchor(cow!("ref")),
            | Token::Alias(cow!("ref")),
            | Token::DocumentEnd,
            | Token::StreamEnd,
            @ None
        );

        assert_eq!(s.stats, stats_of(data));
    }

    #[test]
    fn eat_whitespace()
    {
        let data = "   abc";
        let mut s = Scanner::new(data);

        s.eat_whitespace(false);

        assert_eq!(s.buffer, "abc");
        assert_eq!(s.stats, (3, 0, 3))
    }

    #[test]
    fn eat_whitespace_none()
    {
        let data = "abc";
        let mut s = Scanner::new(data);

        s.eat_whitespace(false);

        assert_eq!(s.buffer, "abc");
        assert_eq!(s.stats, (0, 0, 0))
    }

    /// Calculate what the stats of a given slice should be
    fn stats_of(base: &str) -> MStats
    {
        let mut buffer = base;
        let mut stats = MStats::new();

        loop
        {
            if check!(~buffer => [])
            {
                break;
            }
            else if isBlank!(~buffer)
            {
                advance!(buffer, :stats, 1);
            }
            else if isBreak!(~buffer)
            {
                advance!(buffer, :stats, @line);
            }
            else
            {
                let skip = match buffer.as_bytes()[0]
                {
                    o if (o & 0x80) == 0x00 => 1,
                    o if (o & 0xE0) == 0xC0 => 2,
                    o if (o & 0xF0) == 0xE0 => 3,
                    o if (o & 0xF8) == 0xF0 => 4,
                    _ => unreachable!(),
                };

                advance!(buffer, :stats, skip);
            }
        }

        stats
    }
}
