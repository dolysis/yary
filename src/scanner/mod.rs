mod error;
#[macro_use]
mod macros;

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

        self.eat_whitespace(true);

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
    fn eat_whitespace(&mut self, comments: bool) -> usize
    {
        let mut slice = self.buffer.bytes().enumerate().peekable();
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
            advance!(self.buffer, index);
        }

        // Handle EOF
        //
        // if we hit this, then we didn't get a chance to set
        // chomped in the while loop
        if slice.peek().is_none()
        {
            chomped = self.buffer.len().into();
            self.buffer = ""
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
        if !self.buffer.starts_with('%')
        {
            return Ok(None);
        }

        // Safety: we check above that we have len >= 1 (e.g a '%')
        let kind = DirectiveKind::new(&self.buffer[1..])?;

        // '%' + 'YAML' or 'TAG'
        advance!(self.buffer, 1 + kind.len());

        let token = match kind
        {
            DirectiveKind::Version =>
            {
                // Chomp any preceding whitespace
                self.eat_whitespace(false);

                // Parse the major and minor version numbers

                let (major, rest) = self
                    .buffer
                    .as_bytes()
                    .split_first()
                    .ok_or(ScanError::MissingMajor)
                    .and_then(|(major, rest)| Ok((as_ascii_digit(major)?, rest)))?;

                let minor = rest
                    .get(1)
                    .ok_or(ScanError::MissingMinor)
                    .and_then(as_ascii_digit)?;

                // %YAML 1.1
                //       ^^^
                advance!(self.buffer, 3);

                Token::VersionDirective(major, minor)
            },
            DirectiveKind::Tag => todo!(),
        };

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
enum StreamState
{
    Start,
    Stream,
    Done,
}

#[inline]
fn as_ascii_digit(d: &u8) -> Result<u8>
{
    if !d.is_ascii_digit()
    {
        return Err(ScanError::InvalidVersion);
    }

    Ok(d - 48)
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
    fn directive_yaml()
    {
        let data = "%YAML   1.1 # a comment\n";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::VersionDirective(1, 1)             => "expected version directive",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );
    }

    #[test]
    fn eat_whitespace()
    {
        let data = "   abc";
        let mut s = Scanner::new(data);

        s.eat_whitespace(false);

        assert_eq!(s.buffer, "abc");
    }

    #[test]
    fn eat_whitespace_none()
    {
        let data = "abc";
        let mut s = Scanner::new(data);

        s.eat_whitespace(false);

        assert_eq!(s.buffer, "abc");
    }
}
