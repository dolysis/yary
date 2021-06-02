use crate::token::{ScalarStyle, Slice, StreamEncoding, Token};

#[derive(Debug)]
struct Scanner<'a> {
    buffer: &'a str,
    state: StreamState,
}

impl<'a> Scanner<'a> {
    pub fn new(data: &'a str) -> Self {
        Self {
            buffer: data,
            state: StreamState::Start,
        }
    }

    fn start_stream(&mut self) -> Option<Token<'a>> {
        match self.state {
            StreamState::Start => {
                self.state = StreamState::Stream;

                Some(Token::StreamStart(StreamEncoding::UTF8))
            }
            _ => None,
        }
    }

    fn stream_end(&mut self) -> Option<Token<'a>> {
        match (self.state, self.buffer.is_empty()) {
            (StreamState::Done, _) => None,
            (_, true) => {
                self.state = StreamState::Done;

                Some(Token::StreamEnd)
            }
            (_, false) => None,
        }
    }

    fn eat_whitespace(&mut self) -> usize {
        match self.buffer.find(|c| !char::is_whitespace(c)) {
            Some(pos) => {
                let (_ws, rest) = self.buffer.split_at(pos);
                self.buffer = rest;

                pos
            }
            _ => 0,
        }
    }

    fn document_marker(&mut self) -> Option<Token<'a>> {
        if self.buffer.starts_with("---") {
            self.buffer = split_at(self.buffer, 3);

            Token::DocumentStart.into()
        } else if self.buffer.starts_with("...") {
            self.buffer = split_at(self.buffer, 3);

            Token::DocumentEnd.into()
        } else {
            None
        }
    }
}

#[inline(always)]
fn split_at(b: &str, at: usize) -> &str {
    let (_, rest) = b.split_at(at);
    rest
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(begin) = self.start_stream() {
            return Some(begin);
        }

        self.eat_whitespace();

        if let Some(end) = self.stream_end() {
            return Some(end);
        }

        if let Some(document) = self.document_marker() {
            return Some(document);
        }

        None
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum StreamState {
    Start,
    Stream,
    Done,
}

#[cfg(test)]
mod tests {
    #[macro_use]
    mod macros;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty() {
        let data = "";
        let mut s = Scanner::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );
    }

    #[test]
    fn document_markers() {
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
}
