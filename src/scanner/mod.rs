// Note that this module must come before all others, as
// they depend on the macros which expand into this scope
#[macro_use]
mod macros;

mod error;
mod scalar;
mod tag;

use atoi::atoi;

use self::error::{ScanError, ScanResult as Result};
use crate::{
    scanner::tag::{scan_tag_handle, scan_tag_uri},
    token::{Ref, StreamEncoding, Token},
};

#[derive(Debug)]
struct Scanner<'b>
{
    buffer: &'b str,
    state:  StreamState,
}

impl<'b> Scanner<'b>
{
    pub fn new(data: &'b str) -> Self
    {
        Self {
            buffer: data,
            state:  StreamState::Start,
        }
    }

    fn next_token<'c>(&mut self, scratch: &'c mut Vec<u8>) -> Result<Option<Ref<'b, 'c>>>
    {
        if let Some(begin) = self.start_stream()
        {
            return Ok(begin.borrowed().into());
        }

        Self::eat_whitespace(&mut self.buffer, true);

        if let Some(end) = self.stream_end()
        {
            return Ok(end.borrowed().into());
        }

        if let Some(document) = self.document_marker()
        {
            return Ok(document.borrowed().into());
        }

        if let directive @ Some(_) = self.directive(scratch)?
        {
            return Ok(directive);
        }

        if let Some(anchor) = self.anchor()?
        {
            return Ok(anchor.borrowed().into());
        }

        Ok(None)
    }

    fn start_stream(&mut self) -> Option<Token<'b>>
    {
        match self.state
        {
            StreamState::Start =>
            {
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
    fn eat_whitespace(buffer: &mut &str, comments: bool) -> usize
    {
        let mut slice = buffer.bytes().enumerate().peekable();
        let mut chomped = None;
        let mut chomp_line = false;

        while let Some((index, c)) = slice.next()
        {
            match c
            {
                // Eat spaces
                b' ' =>
                {},
                // If we are starting a comment, chomp the entire line
                b'#' if comments => chomp_line = true,
                // Reset line chomp after eating one
                b'\n' => chomp_line = false,
                // Chomp anything if we're eating the whole line
                _ if chomp_line =>
                {},
                // We're done, encountered a character that isn't whitespace
                _ =>
                {
                    chomped = Some(index);
                    break;
                },
            }
        }

        // Adjust our buffer by the chomped length
        if let Some(index) = chomped
        {
            advance!(*buffer, index);
        }

        // Handle EOF
        //
        // if we hit this, then we didn't get a chance to set
        // chomped in the while loop
        if slice.peek().is_none()
        {
            chomped = buffer.len().into();
            *buffer = ""
        }

        chomped.unwrap_or(0)
    }

    fn document_marker(&mut self) -> Option<Token<'b>>
    {
        if self.buffer.starts_with("---")
        {
            advance!(self.buffer, 3);

            Token::DocumentStart.into()
        }
        else if self.buffer.starts_with("...")
        {
            advance!(self.buffer, 3);

            Token::DocumentEnd.into()
        }
        else
        {
            None
        }
    }

    fn directive<'c>(&mut self, scratch: &'c mut Vec<u8>) -> Result<Option<Ref<'b, 'c>>>
    {
        let mut buffer = self.buffer;

        if !check!(~buffer => b'%')
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
        advance!(buffer, 1 + kind.len());

        let token = match kind
        {
            DirectiveKind::Version =>
            {
                // Chomp any preceding whitespace
                Self::eat_whitespace(&mut buffer, false);

                // %YAML 1.1
                //       ^
                let (major, skip) = scan_directive_version(buffer)?;

                advance!(buffer, skip);

                // %YAML 1.1
                //        ^
                match buffer.as_bytes()
                {
                    [b'.', ..] =>
                    {
                        advance!(buffer, 1);

                        Ok(())
                    },
                    [] => Err(ScanError::UnexpectedEOF),
                    _ => Err(ScanError::InvalidVersion),
                }?;

                // %YAML 1.1
                //         ^
                let (minor, skip) = scan_directive_version(buffer)?;

                advance!(buffer, skip);

                Token::VersionDirective(major, minor).borrowed()
            },
            DirectiveKind::Tag =>
            {
                // Chomp any spaces up to the handle
                Self::eat_whitespace(&mut buffer, false);

                // %TAG !named! :tag:prefix # a comment\n
                //      ^^^^^^^
                let (handle, amt) = match scan_tag_handle(buffer)?
                {
                    Some((handle, amt)) => (handle.into_inner(), amt),
                    None => return Err(ScanError::InvalidTagHandle),
                };
                advance!(buffer, amt);

                // %TAG !named! :tag:prefix # a comment\n
                //             ^
                // Check that there is >= 1 whitespace between handle and
                // prefix
                isBlank!(~buffer, else ScanError::InvalidTagPrefix)?;
                Self::eat_whitespace(&mut buffer, false);

                let mut can_borrow = true;
                // %TAG !named! :tag:prefix # a comment\n
                //              ^^^^^^^^^^^
                let (prefix, amt) = scan_tag_uri(buffer, scratch, &mut can_borrow, false)?;

                // %TAG !named! tag-prefix # a comment\n
                //                        ^
                // Check there is whitespace or a newline after the tag
                isWhiteSpace!(~buffer, amt, else ScanError::InvalidTagPrefix)?;

                // If we can borrow, just take the range directly out of
                // .buffer
                let token = if can_borrow
                {
                    Token::TagDirective(cow!(handle), cow!(&buffer[prefix])).borrowed()
                }
                // Otherwise, we'll need to copy both the handle and prefix, to unify our
                // lifetimes. Note that this isn't strictly necessary, but requiring Token to
                // contain two unrelated lifetimes is just asking for pain and suffering.
                else
                {
                    let start = scratch.len();
                    scratch.extend_from_slice(handle.as_bytes());

                    let handle = std::str::from_utf8(&scratch[start..]).unwrap();
                    let prefix = std::str::from_utf8(&scratch[prefix]).unwrap();

                    Token::TagDirective(cow!(handle), cow!(prefix)).copied()
                };

                advance!(buffer, amt);

                token
            },
        };

        // %YAML 1.1 # some comment\n
        //          ^^^^^^^^^^^^^^^^^ buffer
        // ^^^^^^^^^ self.buffer.len - buffer.len
        advance!(self.buffer, self.buffer.len() - buffer.len());

        Ok(Some(token))
    }

    fn anchor(&mut self) -> Result<Option<Token<'b>>>
    {
        let mut buffer = self.buffer;

        // *anchor 'rest of the line'
        // ^
        let kind = match buffer.as_bytes()
        {
            [b @ b'*', ..] | [b @ b'&', ..] =>
            {
                AnchorKind::new(b).expect("we only bind * or & so this cannot fail")
            },
            _ => return Ok(None),
        };

        advance!(buffer, 1);

        // *anchor 'rest of the line'
        //  ^^^^^^
        let anchor = take_while(buffer.as_bytes(), u8::is_ascii_alphanumeric);

        let anchor = advance!(<- buffer, anchor.len());

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

        // *anchor 'rest of the line'
        //        ^^^^^^^^^^^^^^^^^^^ buffer.len
        // ^^^^^^^ self.buffer.len - buffer.len
        advance!(self.buffer, self.buffer.len() - buffer.len());

        Ok(Some(token))
    }
}

struct ScanIter<'b, 'c>
{
    inner:   &'b mut Scanner<'b>,
    scratch: &'c mut Vec<u8>,
}

impl<'b, 'c> ScanIter<'b, 'c>
{
    pub fn new(inner: &'b mut Scanner<'b>, scratch: &'c mut Vec<u8>) -> Self
    {
        Self { inner, scratch }
    }

    pub fn next(&mut self) -> Option<Result<Ref<'_, '_>>>
    {
        self.inner.next_token(self.scratch).transpose()
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

#[cfg(test)]
mod tests
{
    #[macro_use]
    mod macros;

    use pretty_assertions::assert_eq;

    use super::*;

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
    }

    #[test]
    fn eat_whitespace()
    {
        let data = "   abc";
        let mut s = Scanner::new(data);

        Scanner::eat_whitespace(&mut s.buffer, false);

        assert_eq!(s.buffer, "abc");
    }

    #[test]
    fn eat_whitespace_none()
    {
        let data = "abc";
        let mut s = Scanner::new(data);

        Scanner::eat_whitespace(&mut s.buffer, false);

        assert_eq!(s.buffer, "abc");
    }
}
