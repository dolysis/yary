// Note that this module must come before all others, as
// they depend on the macros which expand into this scope
#[macro_use]
mod macros;

mod context;
mod entry;
mod error;
mod key;
mod scalar;
mod tag;

use std::ops::{Add, AddAssign};

use atoi::atoi;

use self::{
    context::Context,
    entry::TokenEntry,
    error::{ScanError, ScanResult as Result},
    key::Key,
};
use crate::{
    queue::Queue,
    scanner::{
        scalar::flow::scan_flow_scalar,
        tag::{scan_node_tag, scan_tag_directive},
    },
    token::{StreamEncoding, Token},
};

type Tokens<'de> = Queue<TokenEntry<'de>>;

#[derive(Debug)]
struct Scanner
{
    /// Offset into the data buffer to start at
    offset: usize,

    /// Current stream state
    state: StreamState,

    /// Can a simple (i.e not complex) key potentially start
    /// at the current position?
    simple_key_allowed: bool,

    // Subsystems
    stats:   MStats,
    key:     Key,
    context: Context,
}

impl Scanner
{
    pub fn new() -> Self
    {
        Self {
            offset:             0,
            simple_key_allowed: false,
            stats:              MStats::new(),
            state:              StreamState::Start,
            key:                Key::default(),
            context:            Context::new(),
        }
    }

    /// Scan some tokens from the given .base into .tokens
    /// returning the number added.
    pub fn scan_tokens<'de>(&mut self, base: &'de str, tokens: &mut Tokens<'de>) -> Result<usize>
    {
        if let Some(mut buffer) = base
            .get(self.offset..)
            .filter(|_| self.state != StreamState::Done)
        {
            let existing_tokens = tokens.len();

            self.scan_next_token(&mut buffer, tokens)?;

            self.offset = base.len() - buffer.len();

            return Ok(tokens.len() - existing_tokens);
        }

        Ok(0)
    }

    fn scan_next_token<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>)
        -> Result<()>
    {
        if self.state == StreamState::Start
        {
            self.start_stream(tokens);
            return Ok(());
        }

        self.eat_whitespace(base, COMMENTS);

        if base.is_empty() || self.state == StreamState::Done
        {
            return self.stream_end(*base, tokens);
        }

        if self.stats.column == 0
            && isWhiteSpaceZ!(~base, 3)
            && check!(~base => [b'-', b'-', b'-', ..] | [b'.', b'.', b'.', ..])
        {
            self.document_marker(base, tokens);
            return Ok(());
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
        match base.as_bytes()
        {
            [DIRECTIVE, ..] => self.directive(base, tokens),
            [ANCHOR, ..] | [ALIAS, ..] => self.anchor(base, tokens),
            [TAG, ..] => self.tag(base, tokens),
            [SINGLE, ..] | [DOUBLE, ..] => self.flow_scalar(base, tokens),
            [VALUE, ..] if isWhiteSpaceZ!(~base, 1) => self.value(base, tokens),
            [b @ FLOW_MAPPING_START, ..] | [b @ FLOW_SEQUENCE_START, ..] =>
            {
                self.flow_collection_start(base, tokens, *b == FLOW_MAPPING_START)
            },
            [b @ FLOW_MAPPING_END, ..] | [b @ FLOW_SEQUENCE_END, ..] =>
            {
                self.flow_collection_end(base, tokens, *b == FLOW_MAPPING_END)
            },
            [FLOW_ENTRY, ..] => self.flow_collection_entry(base, tokens),
            _ => unreachable!(),
        }
    }

    fn start_stream(&mut self, tokens: &mut Tokens)
    {
        if self.state == StreamState::Start
        {
            // A key is allowed at the beginning of the stream
            self.simple_key_allowed = true;

            self.state = StreamState::Stream;

            let token = Token::StreamStart(StreamEncoding::UTF8);

            enqueue!(token, :self.stats => tokens);
        }
    }

    fn stream_end(&mut self, buffer: &str, tokens: &mut Tokens) -> Result<()>
    {
        match (self.state, buffer.is_empty())
        {
            (StreamState::Done, _) =>
            {},
            (_, true) =>
            {
                self.remove_saved_key()?;
                self.state = StreamState::Done;

                let token = Token::StreamEnd;

                enqueue!(token, :self.stats => tokens);
            },
            (_, false) =>
            {},
        }

        Ok(())
    }

    /// Chomp whitespace and optionally comments until we
    /// reach the next token, updating buffer[0] to the
    /// beginning of the new token
    fn eat_whitespace(&mut self, buffer: &mut &str, comments: bool) -> usize
    {
        let mut stats = MStats::new();

        let amt = eat_whitespace(*buffer, &mut stats, comments);

        // A new line may start a key in the block context
        //
        // FIXME: we don't track flow/block contexts yet, add check
        // here when we do
        if stats.lines != 0
        {
            self.simple_key_allowed = true;
        }

        advance!(*buffer, amt);
        self.stats += stats;

        amt
    }

    fn document_marker(&mut self, buffer: &mut &str, tokens: &mut Tokens)
    {
        if self.stats.column == 0 && isWhiteSpaceZ!(~buffer, 3)
        {
            let token = match buffer.as_bytes()
            {
                [b'-', b'-', b'-', ..] => Token::DocumentStart,
                [b'.', b'.', b'.', ..] => Token::DocumentEnd,
                _ => return,
            };

            advance!(*buffer, :self.stats, 3);

            // A key cannot follow a document marker
            self.simple_key_allowed = false;

            enqueue!(token, :self.stats => tokens);
        }
    }

    fn directive<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        let mut buffer = *base;
        let mut stats = MStats::new();

        if !check!(~buffer => [DIRECTIVE, ..])
        {
            return Ok(());
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

                Token::VersionDirective(major, minor)
            },
            DirectiveKind::Tag =>
            {
                // Chomp any spaces up to the handle
                advance!(buffer, eat_whitespace(buffer, &mut stats, !COMMENTS));

                // Scan the directive, copying if necessary
                let (token, amt) = scan_tag_directive(buffer, &mut stats)?;
                advance!(buffer, amt);

                token
            },
        };

        // A key cannot follow a directive (a newline is required)
        self.simple_key_allowed = false;

        // %YAML 1.1 # some comment\n
        //          ^^^^^^^^^^^^^^^^^ buffer
        // ^^^^^^^^^ self.buffer.len - buffer.len
        advance!(*base, base.len() - buffer.len());
        self.stats += stats;

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    /// Try eat a tag, returning a Token if one could be
    /// found at the current buffer head, or none if one
    /// couldn't.
    fn tag<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        let mut buffer = *base;
        let mut stats = MStats::new();

        if !check!(~buffer => [TAG, ..])
        {
            return Ok(());
        }

        self.save_key(!REQUIRED)?;

        let (token, amt) = scan_node_tag(buffer, &mut stats)?;
        advance!(buffer, amt);

        // A key may not start after a tag (only before)
        self.simple_key_allowed = false;

        // !named_tag!type-suffix "my tagged value"
        //                       ^^^^^^^^^^^^^^^^^^ buffer
        // ^^^^^^^^^^^^^^^^^^^^^^ self.buffer.len - buffer.len
        advance!(*base, base.len() - buffer.len());
        self.stats += stats;

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn anchor<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        let mut buffer = *base;
        let mut stats = MStats::new();

        // *anchor 'rest of the line'
        // ^
        let kind = match buffer.as_bytes()
        {
            [b @ ALIAS, ..] | [b @ ANCHOR, ..] =>
            {
                AnchorKind::new(b).expect("we only bind * or & so this cannot fail")
            },
            _ => return Ok(()),
        };

        self.save_key(!REQUIRED)?;

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

        // A key may not start after an anchor (only before)
        self.simple_key_allowed = false;

        // *anchor 'rest of the line'
        //        ^^^^^^^^^^^^^^^^^^^ buffer.len
        // ^^^^^^^ self.buffer.len - buffer.len
        advance!(*base, base.len() - buffer.len());
        self.stats += stats;

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn flow_scalar<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        let buffer = *base;
        let mut stats = MStats::new();
        let single = check!(~buffer => [SINGLE, ..]);

        if !check!(~buffer => [SINGLE, ..] | [DOUBLE, ..])
        {
            return Ok(());
        }

        self.save_key(!REQUIRED)?;

        let (range, amt) = scan_flow_scalar(buffer, &mut stats, single)?;
        let token = range.into_token(buffer)?;

        // If we found a key, save the scalar
        // FIXME: we need to allow Scanner callers to indicate
        // whether the buffer they provide is growable
        const EXTENDABLE: bool = false;
        if let Some(saved) = self.key.saved().take()
        {
            if check_is_key(&buffer[amt..], &stats, saved.key().required(), EXTENDABLE)?
            {
                let key_stats = saved.stats();
                // Roll the indent and push a block mapping start token if
                // necessary
                roll_indent(
                    &mut self.context,
                    key_stats.read,
                    tokens,
                    key_stats.column,
                    BLOCK_MAP,
                )?;

                // Then push a token to the queue
                enqueue!(Token::Key, :key_stats => tokens);
            }
        }

        // A key cannot follow a flow scalar, as we're either
        // currently in a key (which should be followed by a
        // value), or a value which needs a separator (e.g line
        // break) before another key is legal
        self.simple_key_allowed = false;

        advance!(*base, amt);
        self.stats += stats;

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn value<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        let mut buffer = *base;
        let mut stats = MStats::new();

        if !(check!(~buffer => [VALUE, ..]) && isWhiteSpaceZ!(~buffer, 1))
        {
            return Ok(());
        }

        let token = Token::Value;
        advance!(buffer, :stats, 1);

        // Simple keys may start after a ':' in the block context
        self.simple_key_allowed = self.context.is_block();

        advance!(*base, base.len() - buffer.len());
        self.stats += stats;

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn flow_collection_start<'de>(
        &mut self,
        base: &mut &'de str,
        tokens: &mut Tokens<'de>,
        map: bool,
    ) -> Result<()>
    {
        let token = match map
        {
            true => Token::FlowMappingStart,
            false => Token::FlowSequenceStart,
        };

        self.context.flow_increment()?;

        advance!(*base, :self.stats, 1);

        enqueue!(token, :self.stats => tokens);

        self.save_key(!REQUIRED)?;

        // A simple key may start after '[' or '{'
        self.simple_key_allowed = true;

        Ok(())
    }

    fn flow_collection_end<'de>(
        &mut self,
        base: &mut &'de str,
        tokens: &mut Tokens<'de>,
        map: bool,
    ) -> Result<()>
    {
        let token = match map
        {
            true => Token::FlowMappingEnd,
            false => Token::FlowSequenceEnd,
        };

        // Reset saved key
        self.remove_saved_key()?;

        // Decrease flow level by 1
        self.context.flow_decrement()?;

        // A simple key is not allowed after a ']' or '}'
        self.simple_key_allowed = false;

        advance!(*base, :self.stats, 1);

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn flow_collection_entry<'de>(
        &mut self,
        base: &mut &'de str,
        tokens: &mut Tokens<'de>,
    ) -> Result<()>
    {
        // Reset saved key
        self.remove_saved_key()?;

        // A simple key can start after a ','
        self.simple_key_allowed = true;

        advance!(*base, :self.stats, 1);

        let token = Token::FlowEntry;

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn block_collection_entry<'de>(
        &mut self,
        base: &mut &'de str,
        tokens: &mut Tokens<'de>,
    ) -> Result<()>
    {
        match self.context.is_block() && self.simple_key_allowed
        {
            true => roll_indent(
                &mut self.context,
                self.stats.read,
                tokens,
                self.stats.column,
                !BLOCK_MAP,
            ),
            false => Err(ScanError::InvalidBlockEntry),
        }?;

        // Reset saved key
        self.remove_saved_key()?;

        // A key is possible after a '-'
        self.simple_key_allowed = true;

        advance!(*base, :self.stats, 1);

        let token = Token::BlockEntry;
        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    /// Save a position in the buffer as a potential simple
    /// key location, if a simple key is possible
    fn save_key(&mut self, required: bool) -> Result<()>
    {
        // A key is required if we are in the block context, and the
        // current column equals the indentation level
        let required =
            required || (self.context.is_block() && self.context.indent() == self.stats.column);

        if self.simple_key_allowed
        {
            self.remove_saved_key()?;

            self.key.save(self.stats.clone(), required)
        }

        Ok(())
    }

    fn remove_saved_key(&mut self) -> Result<()>
    {
        if let Some(saved) = self.key.saved().take()
        {
            if saved.key().required()
            {
                return Err(ScanError::MissingValue);
            }
        }

        Ok(())
    }

    fn is_key_required(&mut self) -> Result<()>
    {
        if self.key.required()
        {
            return Err(ScanError::MissingValue);
        }

        Ok(())
    }

    fn key_forbidden(&mut self)
    {
        self.key.forbidden()
    }
}

struct ScanIter<'de>
{
    data:   &'de str,
    scan:   Scanner,
    tokens: Tokens<'de>,

    done: bool,
}

impl<'de> ScanIter<'de>
{
    pub fn new(data: &'de str) -> Self
    {
        Self {
            data,
            scan: Scanner::new(),
            tokens: Tokens::new(),
            done: false,
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token<'de>>>
    {
        if (!self.done) && self.tokens.is_empty()
        {
            if let 0 = self.scan.scan_tokens(self.data, &mut self.tokens)?
            {
                self.done = true
            }
        }

        if !self.done
        {
            Ok(self.tokens.pop().map(|e| e.into_token()))
        }
        else
        {
            Ok(None)
        }
    }
}

impl<'de> Iterator for ScanIter<'de>
{
    type Item = Result<Token<'de>>;

    fn next(&mut self) -> Option<Self::Item>
    {
        dbg!(self.next_token().transpose())
    }
}

impl<'de> std::iter::FusedIterator for ScanIter<'de> {}

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

fn check_is_key(buffer: &str, key_stats: &MStats, required: bool, extendable: bool)
    -> Result<bool>
{
    let mut stats = key_stats.clone();
    let amt = eat_whitespace(buffer, &mut stats, !COMMENTS);

    /*
     * The YAML spec requires that implicit keys are
     *
     * 1. Limited to a single line
     * 2. Must be less than 1024 characters, including
     *    trailing whitespace to a ': '
     *
     * https://yaml.org/spec/1.2/spec.html#ns-s-implicit-yaml-key(c)
     */
    if stats.lines > 0 || stats.read > 1024
    {
        if required
        {
            return Err(ScanError::MissingValue);
        }

        return Ok(false);
    }

    // If the buffer has the possibility to be grown, we should
    // error here, as it is possible we've hit a read
    // boundary
    if extendable && buffer[amt..].len() < 2
    {
        return Err(ScanError::UnexpectedEOF);
    }

    let is_key = check!(~buffer, amt => [VALUE, ..]) && isWhiteSpaceZ!(~buffer, 1);

    Ok(is_key)
}

/// Roll the indentation level and push a block collection
/// indent token to the indent stack if required
fn roll_indent<'de>(
    context: &mut Context,
    mark: usize,
    tokens: &mut Tokens<'de>,
    column: usize,
    map: bool,
) -> Result<()>
{
    if context.is_block() && context.indent() < column
    {
        context.indent_increment(column)?;

        let token = match map
        {
            true => Token::BlockMappingStart,
            false => Token::BlockSequenceStart,
        };

        enqueue!(token, mark => tokens);
    }

    Ok(())
}

/// Unroll indentation level until we reach .column, pushing
/// a block collection unindent token for every stored
/// indent level
fn unroll_indent<'de>(
    context: &mut Context,
    mark: usize,
    tokens: &mut Tokens<'de>,
    column: usize,
) -> Result<()>
{
    if context.is_block()
    {
        let generator = |_| {
            let token = Token::BlockEnd;
            enqueue!(token, mark => tokens);

            Ok(())
        };

        context.indent_decrement(column, generator)?;
    }

    Ok(())
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
const FLOW_MAPPING_START: u8 = b'{';
const FLOW_MAPPING_END: u8 = b'}';
const FLOW_SEQUENCE_START: u8 = b'[';
const FLOW_SEQUENCE_END: u8 = b']';
const FLOW_ENTRY: u8 = b',';

const COMMENTS: bool = true;
const REQUIRED: bool = true;
const BLOCK_MAP: bool = true;
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
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn multi_document_empty()
    {
        let data = "---\n---\n---";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8),
            | Token::DocumentStart,
            | Token::DocumentStart,
            | Token::DocumentStart,
            | Token::StreamEnd,
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
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::DocumentStart                      => "expected start of document",
            | Token::DocumentEnd                        => "expected end of document",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn flow_collection_mapping_indicators()
    {
        let data = "{}";
        let mut s = ScanIter::new(data);

        // Note that the doubled braces here are because of Rust's
        // fmtstr escaping rules.
        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::FlowMappingStart                   => "expected a flow mapping start '{{'",
            | Token::FlowMappingEnd                     => "expected a flow mapping end '}}'",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn flow_collection_sequence_indicators()
    {
        let data = "[]";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::FlowSequenceStart                  => "expected a flow sequence start '['",
            | Token::FlowSequenceEnd                    => "expected a flow sequence end ']'",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn flow_collection_mapping()
    {
        use ScalarStyle::SingleQuote;
        let data = "{'a key': 'a value','another key': 'another value'}";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            | Token::FlowMappingStart                           => "expected a flow mapping start '{{'",
            | Token::Key                                        => "expected a key",
            | Token::Scalar(cow!("a key"), SingleQuote)         => "expected a scalar key: 'a key'",
            | Token::Value                                      => "expected a value",
            | Token::Scalar(cow!("a value"), SingleQuote)       => "expected a scalar value: 'a value'",
            | Token::FlowEntry                                  => "expected a flow entry: ','",
            | Token::Key                                        => "expected a key",
            | Token::Scalar(cow!("another key"), SingleQuote)   => "expected a scalar key: 'another key'",
            | Token::Value                                      => "expected a value",
            | Token::Scalar(cow!("another value"), SingleQuote) => "expected a scalar value: 'another value'",
            | Token::FlowMappingEnd                             => "expected a flow mapping end '}}'",
            | Token::StreamEnd                                  => "expected end of stream",
            @ None                                              => "expected stream to be finished"
        );
    }

    #[test]
    fn flow_collection_sequence()
    {
        use ScalarStyle::SingleQuote;
        let data = "['a key': 'a value','another key': 'another value']";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            | Token::FlowSequenceStart                          => "expected a flow sequence start '['",
            | Token::Key                                        => "expected a key",
            | Token::Scalar(cow!("a key"), SingleQuote)         => "expected a scalar key: 'a key'",
            | Token::Value                                      => "expected a value",
            | Token::Scalar(cow!("a value"), SingleQuote)       => "expected a scalar value: 'a value'",
            | Token::FlowEntry                                  => "expected a flow entry: ','",
            | Token::Key                                        => "expected a key",
            | Token::Scalar(cow!("another key"), SingleQuote)   => "expected a scalar key: 'another key'",
            | Token::Value                                      => "expected a value",
            | Token::Scalar(cow!("another value"), SingleQuote) => "expected a scalar value: 'another value'",
            | Token::FlowSequenceEnd                            => "expected a flow sequence end ']'",
            | Token::StreamEnd                                  => "expected end of stream",
            @ None                                              => "expected stream to be finished"
        );
    }

    #[test]
    fn flow_collection_nested()
    {
        use ScalarStyle::SingleQuote;
        let data = "[
            {'a map': 'of values'},
            {'inside': 'a sequence'},
            'a string',
            ['and', 'lists','of', 'strings'],
            {'wow': {'this': {'nesting': {'goes': ['deep', '!']}}}}
        ]";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8),
            | Token::FlowSequenceStart,
            | Token::FlowMappingStart,
            | Token::Key,
            | Token::Scalar(cow!("a map"), SingleQuote),
            | Token::Value,
            | Token::Scalar(cow!("of values"), SingleQuote),
            | Token::FlowMappingEnd,
            | Token::FlowEntry,
            | Token::FlowMappingStart,
            | Token::Key,
            | Token::Scalar(cow!("inside"), SingleQuote),
            | Token::Value,
            | Token::Scalar(cow!("a sequence"), SingleQuote),
            | Token::FlowMappingEnd,
            | Token::FlowEntry,
            | Token::Scalar(cow!("a string"), SingleQuote),
            | Token::FlowEntry,
            | Token::FlowSequenceStart,
            | Token::Scalar(cow!("and"), SingleQuote),
            | Token::FlowEntry,
            | Token::Scalar(cow!("lists"), SingleQuote),
            | Token::FlowEntry,
            | Token::Scalar(cow!("of"), SingleQuote),
            | Token::FlowEntry,
            | Token::Scalar(cow!("strings"), SingleQuote),
            | Token::FlowSequenceEnd,
            | Token::FlowEntry,
            | Token::FlowMappingStart,
            | Token::Key,
            | Token::Scalar(cow!("wow"), SingleQuote),
            | Token::Value,
            | Token::FlowMappingStart,
            | Token::Key,
            | Token::Scalar(cow!("this"), SingleQuote),
            | Token::Value,
            | Token::FlowMappingStart,
            | Token::Key,
            | Token::Scalar(cow!("nesting"), SingleQuote),
            | Token::Value,
            | Token::FlowMappingStart,
            | Token::Key,
            | Token::Scalar(cow!("goes"), SingleQuote),
            | Token::Value,
            | Token::FlowSequenceStart,
            | Token::Scalar(cow!("deep"), SingleQuote),
            | Token::FlowEntry,
            | Token::Scalar(cow!("!"), SingleQuote),
            | Token::FlowSequenceEnd,
            | Token::FlowMappingEnd,
            | Token::FlowMappingEnd,
            | Token::FlowMappingEnd,
            | Token::FlowMappingEnd,
            | Token::FlowSequenceEnd,
            | Token::StreamEnd,
            @ None
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn chomp_comments()
    {
        let data = "  # a comment\n\n#one two three\n       #four!";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn comment_in_document_markers()
    {
        let data = "---\n# abcdefg \n  # another comment     \n...";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::DocumentStart                      => "expected start of document",
            | Token::DocumentEnd                        => "expected end of document",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn directive_version()
    {
        let data = "%YAML   1.1 # a comment\n";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::VersionDirective(1, 1)             => "expected version directive (1, 1)",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn directive_version_large()
    {
        let data = "%YAML   121.80 # a comment\n";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::VersionDirective(121, 80)          => "expected version directive (121, 80)",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn directive_version_invalid()
    {
        let data = "%YAML   foo.bar # a comment\n";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
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
            | Token::StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
            | Token::TagDirective(cow!("!named!"), cow!("my:cool:tag")) => "expected named tag directive",
            | Token::StreamEnd                                          => "expected end of stream",
            @ None                                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn directive_tag_primary()
    {
        let data = "%TAG ! my:cool:tag\n";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)              => "expected start of stream",
            | Token::TagDirective(cow!("!"), cow!("my:cool:tag"))   => "expected primary tag directive",
            | Token::StreamEnd                                      => "expected end of stream",
            @ None                                                  => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn directive_tag_secondary()
    {
        let data = "%TAG !! @my/crazy&tag:  \n";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
            | Token::TagDirective(cow!("!!"), cow!("@my/crazy&tag:"))   => "expected secondary tag directive",
            | Token::StreamEnd                                          => "expected end of stream",
            @ None                                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn directive_tag_ending_ws()
    {
        let data = "%TAG !! @my/crazy&tag:";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
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
            | Token::StreamStart(StreamEncoding::UTF8)                  => "expected start of stream",
            | Token::TagDirective(cow!("!!"), cow!(":My:Ɔrazy:TȂg:"))   => "expected unescaped unicode prefix",
            | Token::StreamEnd                                          => "expected end of stream",
            @ None                                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn anchor_alias()
    {
        let data = "*alias\n";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Alias(cow!("alias"))               => "expected an alias named 'alias'",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn anchor()
    {
        let data = "    &anchor     \n";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Anchor(cow!("anchor"))             => "expected an anchor named 'anchor'",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn tag_primary()
    {
        let data = "!a ";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Tag(cow!("!"), cow!("a"))          => "expected a primary tag ('!', 'a')",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn tag_secondary()
    {
        let data = "!!str ";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Tag(cow!("!!"), cow!("str"))       => "expected a secondary tag ('!!', 'str')",
            | Token::StreamEnd                          => "expected end of stream",
            @ None                                      => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn tag_named()
    {
        let data = "    !named!tag-suffix ";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            | Token::Tag(cow!("!named!"), cow!("tag-suffix"))   => "expected a global tag ('!named!', 'tag-suffix')",
            | Token::StreamEnd                                  => "expected end of stream",
            @ None                                              => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn tag_escaped()
    {
        let data = "!n!my:%3D%3descaped: ";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            | Token::Tag(cow!("!n!"), cow!("my:==escaped:"))    => "expected a global tag ('!n!', 'my:==escaped:')",
            | Token::StreamEnd                                  => "expected end of stream",
            @ None                                              => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn tag_non_resolving()
    {
        let data = "! ";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)          => "expected start of stream",
            | Token::Tag(cow!("!"), cow!(""))                   => "expected a non resolving tag ('!', '')",
            | Token::StreamEnd                                  => "expected end of stream",
            @ None                                              => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, (2, 0, 2));
    }

    #[test]
    fn flow_scalar_single_simple()
    {
        use ScalarStyle::SingleQuote;

        let data = "'hello world, single quoted flow scalar'";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                                      => "expected start of stream",
            | Token::Scalar(cow!("hello world, single quoted flow scalar"), SingleQuote)    => "expected a flow scalar (single)",
            | Token::StreamEnd                                                              => "expected end of stream",
            @ None                                                                          => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn flow_scalar_single_complex()
    {
        use ScalarStyle::SingleQuote;

        let data = "'line0
            line1
            
            line3
            line4'";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Token::Scalar(cow!("line0 line1\nline3 line4"), SingleQuote)  => "expected a flow scalar (single)",
            | Token::StreamEnd                                              => "expected end of stream",
            @ None                                                          => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn flow_scalar_double_simple()
    {
        use ScalarStyle::DoubleQuote;

        let data = r#""line0 line1\nline3\tline4""#;
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Token::Scalar(cow!("line0 line1\nline3\tline4"), DoubleQuote) => "expected a flow scalar (double)",
            | Token::StreamEnd                                              => "expected end of stream",
            @ None                                                          => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
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
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Token::Scalar(cow!("line0 line1\nline3 line4"), DoubleQuote)  => "expected a flow scalar (double)",
            | Token::StreamEnd                                              => "expected end of stream",
            @ None                                                          => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
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
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)                      => "expected start of stream",
            | Token::Tag(cow!("!!"), cow!("str"))                           => "expected a secondary tag ('!!', 'str')",
            | Token::Scalar(cow!("line0 line1\nline3 line4"), DoubleQuote)  => "expected a flow scalar (double)",
            | Token::StreamEnd                                              => "expected end of stream",
            @ None                                                          => "expected stream to be finished"
        );

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn key_simple_style_single()
    {
        use ScalarStyle::SingleQuote;

        let data = "'key': ";
        let mut s = ScanIter::new(data);

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
    fn key_many_style_single()
    {
        use ScalarStyle::SingleQuote;

        let data = "'key1': 'value1'\n'key2': 'value2'";
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8)  => "expected start of stream",
            | Token::Key                                => "expected an implicit key",
            | Token::Scalar(cow!("key1"), SingleQuote)  => "expected a flow scalar (single)",
            | Token::Value                              => "expected a value token",
            | Token::Scalar(cow!("value1"), SingleQuote)=> "expected a flow scalar (single)",
            | Token::Key                                => "expected an implicit key",
            | Token::Scalar(cow!("key2"), SingleQuote)  => "expected a flow scalar (single)",
            | Token::Value                              => "expected a value token",
            | Token::Scalar(cow!("value2"), SingleQuote)=> "expected a flow scalar (single)",
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
        let mut s = ScanIter::new(data);

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

        assert_eq!(s.scan.stats, stats_of(data));
    }

    #[test]
    fn complex_no_map_sequence()
    {
        use ScalarStyle::{DoubleQuote, SingleQuote};

        let data = r##"

%YAML           1.2                     # our document's version.
%TAG !          primary:namespace       # our doc's primary tag
%TAG !!         secondary/namespace:    # our doc's secondary tag
%TAG !named0!   named0:                 # A named tag
---

!!str "an anchor": &ref !value 'some   
                                value'
!!str 'an alias': *ref

...

"##;
        let mut s = ScanIter::new(data);

        tokens!(s =>
            | Token::StreamStart(StreamEncoding::UTF8),
            | Token::VersionDirective(1, 2),
            | Token::TagDirective(cow!("!"), cow!("primary:namespace")),
            | Token::TagDirective(cow!("!!"), cow!("secondary/namespace:")),
            | Token::TagDirective(cow!("!named0!"), cow!("named0:")),
            | Token::DocumentStart,
            | Token::Tag(cow!("!!"), cow!("str")),
            | Token::Key,
            | Token::Scalar(cow!("an anchor"), DoubleQuote),
            | Token::Value,
            | Token::Anchor(cow!("ref")),
            | Token::Tag(cow!("!"), cow!("value")),
            | Token::Scalar(cow!("some value"), SingleQuote),
            | Token::Tag(cow!("!!"), cow!("str")),
            | Token::Key,
            | Token::Scalar(cow!("an alias"), SingleQuote),
            | Token::Value,
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
        let mut buffer = data;
        let mut s = Scanner::new();

        s.eat_whitespace(&mut buffer, false);

        assert_eq!(buffer, "abc");
        assert_eq!(s.stats, (3, 0, 3))
    }

    #[test]
    fn eat_whitespace_none()
    {
        let data = "abc";
        let mut buffer = data;
        let mut s = Scanner::new();

        s.eat_whitespace(&mut buffer, false);

        assert_eq!(buffer, "abc");
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
