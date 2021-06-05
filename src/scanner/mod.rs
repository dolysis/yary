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

    fn eat_whitespace(&mut self) -> usize
    {
        let mut slice = self.buffer.char_indices().peekable();
        let mut chomped = None;
        let mut chomp_line = false;

        while let Some((index, c)) = slice.next()
        {
            match c
            {
                // Eat spaces
                ' ' =>
                {},
                // If we are starting a comment, chomp the entire line
                '#' => chomp_line = true,
                // Reset line chomp after eating one
                '\n' => chomp_line = false,
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
            self.buffer = split_at(self.buffer, index)
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
            self.buffer = split_at(self.buffer, 3);

            Token::DocumentStart.into()
        }
        else if self.buffer.starts_with("...")
        {
            self.buffer = split_at(self.buffer, 3);

            Token::DocumentEnd.into()
        }
        else
        {
            None
        }
    }
}

#[inline(always)]
fn split_at(b: &str, at: usize) -> &str
{
    let (_, rest) = b.split_at(at);
    rest
}

impl<'a> Iterator for Scanner<'a>
{
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item>
    {
        if let Some(begin) = self.start_stream()
        {
            return Some(begin);
        }

        self.eat_whitespace();

        if let Some(end) = self.stream_end()
        {
            return Some(end);
        }

        if let Some(document) = self.document_marker()
        {
            return Some(document);
        }

        None
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum StreamState
{
    Start,
    Stream,
    Done,
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
}
