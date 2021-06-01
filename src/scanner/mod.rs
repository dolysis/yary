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
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(begin) = self.start_stream() {
            return Some(begin);
        }

        if let Some(end) = self.stream_end() {
            return Some(end);
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
}
