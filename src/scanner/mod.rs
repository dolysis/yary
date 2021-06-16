// Note that this module must come before all others, as
// they depend on the macros which expand into this scope
#[macro_use]
mod macros;

mod error;
mod scalar;

use atoi::atoi;

use self::error::{ScanError, ScanResult as Result};
use crate::token::{StreamEncoding, Token};

#[derive(Debug)]
struct Scanner<'a>
{
    buffer: &'a str,
    state:  StreamState,
}

impl<'a> Scanner<'a>
{
    pub fn new(data: &'a str) -> Self
    {
        Self {
            buffer: data,
            state:  StreamState::Start,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token<'a>>>
    {
        if let begin @ Some(_) = self.start_stream()
        {
            return Ok(begin);
        }

        Self::eat_whitespace(&mut self.buffer, true);

        if let end @ Some(_) = self.stream_end()
        {
            return Ok(end);
        }

        if let document @ Some(_) = self.document_marker()
        {
            return Ok(document);
        }

        if let directive @ Some(_) = self.directive()?
        {
            return Ok(directive);
        }

        if let anchor @ Some(_) = self.anchor()?
        {
            return Ok(anchor);
        }

        Ok(None)
    }

    fn start_stream(&mut self) -> Option<Token<'a>>
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

    fn stream_end(&mut self) -> Option<Token<'a>>
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

    fn document_marker(&mut self) -> Option<Token<'a>>
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

    fn directive(&mut self) -> Result<Option<Token<'a>>>
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

                Token::VersionDirective(major, minor)
            },
            DirectiveKind::Tag =>
            {
                let mut markers = 0;

                // Chomp any spaces up to the handle
                Self::eat_whitespace(&mut buffer, false);

                // %TAG !handle! tag-prefix # a comment \n
                //      ^
                check!(~buffer => b'!', else ScanError::InvalidTagHandle)?;

                markers += 1;

                // %TAG !handle! tag-prefix # a comment \n
                //       ^^^^^^
                // Safety: we just proved above we have >= 1 byte ('!')
                let name = take_while(buffer[1..].as_bytes(), u8::is_ascii_alphanumeric);

                match buffer.as_bytes().get(markers + name.len())
                {
                    // %TAG !! tag-prefix # a comment \n
                    //       ^
                    // Either a secondary handle (!!) or named (!:alphanumeric:!)
                    Some(b'!') => markers += 1,
                    // %TAG ! tag-prefix # a comment \n
                    //       ^
                    // If no name, and no second ! this is a primary handle
                    _ if name.is_empty() =>
                    {},
                    // Otherwise its an error
                    Some(_) => return Err(ScanError::InvalidTagHandle),
                    None => return Err(ScanError::UnexpectedEOF),
                }

                let handle = advance!(<- buffer, markers + name.len());

                // Check that there is >= 1 whitespace between handle and
                // prefix
                check!(~buffer => b' ', else ScanError::InvalidTagPrefix)?;

                Self::eat_whitespace(&mut buffer, false);

                // %TAG !named! :tag:prefix # a comment\n
                //              ^^^^^^^^^^^
                let prefix = match scan_directive_tag_prefix(buffer.as_bytes())
                {
                    [] => return Err(ScanError::InvalidTagPrefix),
                    prefix @ [..] => prefix,
                };

                let prefix = advance!(<- buffer, prefix.len());

                // %TAG !named! tag-prefix # a comment\n
                //                        ^
                // Check there is whitespace or a newline after the tag
                check!(~buffer => b' ' | b'\n', else ScanError::InvalidTagPrefix)?;

                Token::TagDirective(cow!(handle), cow!(prefix))
            },
        };

        // %YAML 1.1 # some comment\n
        //          ^^^^^^^^^^^^^^^^^ buffer
        // ^^^^^^^^^ self.buffer.len - buffer.len
        advance!(self.buffer, self.buffer.len() - buffer.len());

        Ok(Some(token))
    }

    fn anchor(&mut self) -> Result<Option<Token<'a>>>
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

impl<'a> Iterator for Scanner<'a>
{
    type Item = Result<Token<'a>>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.next_token().transpose()
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

fn scan_directive_tag_prefix(b: &[u8]) -> &[u8]
{
    take_while(b, valid_in_tag_prefix)
}

fn valid_in_tag_prefix(b: &u8) -> bool
{
    assert_ne!(*b, b'%', "FIXME: url escape decode not implemented yet!");

    matches!(
        *b,
        // alphanumeric
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' |
        // !, $, &, ', (, ), *, +, -, ., /, :, ;
        b'!' | b'$' | b'&'..=b'/' | b':' | b';' |
        // =, ?, @, _, ~
        b'=' | b'?' | b'@' | b'_' | b'~'
    )
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
