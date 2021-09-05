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
    context::{Context, Indent},
    entry::TokenEntry,
    error::{ScanError, ScanResult as Result},
    key::{Key, KeyPossible},
    scalar::{block::scan_block_scalar, plain::scan_plain_scalar},
};
use crate::{
    queue::Queue,
    scanner::{
        context::STARTING_INDENT,
        scalar::flow::scan_flow_scalar,
        tag::{scan_node_tag, scan_tag_directive},
    },
    token::{Marker, StreamEncoding, Token},
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
        let mut num_tokens = 0;
        let starting_tokens = tokens.len();

        while self.state != StreamState::Done
            && (starting_tokens == tokens.len() || self.key.possible())
        {
            if let Some(mut buffer) = base.get(self.offset..)
            {
                self.scan_next_token(&mut buffer, tokens)?;

                self.offset = base.len() - buffer.len();

                num_tokens = tokens.len() - starting_tokens;
            }
        }

        Ok(num_tokens)
    }

    fn scan_next_token<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>)
        -> Result<()>
    {
        // Is it the beginning of the stream?
        if self.state == StreamState::Start
        {
            self.start_stream(tokens);
            return Ok(());
        }

        // Eat whitespace to the next delimiter
        self.eat_whitespace(base, COMMENTS);

        // Remove any saved key positions that cannot contain keys
        // anymore
        self.expire_stale_saved_key()?;

        // Handle indentation unrolling
        self.pop_zero_indent_sequence(*base, tokens)?;
        self.unroll_indent(tokens, self.stats.column)?;

        // Is it the end of a stream?
        if base.is_empty() || self.state == StreamState::Done
        {
            return self.stream_end(*base, tokens);
        }

        // Fetch the next token(s)
        match base.as_bytes()
        {
            // Is it a directive?
            [DIRECTIVE, ..] if self.stats.column == 0 => self.directive(base, tokens),

            // Is it a document marker?
            [b @ b'-', b'-', b'-', ..] | [b @ b'.', b'.', b'.', ..]
                if self.stats.column == 0 && isWhiteSpaceZ!(~base, 3) =>
            {
                self.document_marker(base, tokens, *b == b'-')
            },

            // Is it the start of a flow collection?
            [b @ FLOW_MAPPING_START, ..] | [b @ FLOW_SEQUENCE_START, ..] =>
            {
                self.flow_collection_start(base, tokens, *b == FLOW_MAPPING_START)
            },

            // Is it the end of a flow collection?
            [b @ FLOW_MAPPING_END, ..] | [b @ FLOW_SEQUENCE_END, ..] =>
            {
                self.flow_collection_end(base, tokens, *b == FLOW_MAPPING_END)
            },

            // Is a flow collection entry?
            [FLOW_ENTRY, ..] => self.flow_collection_entry(base, tokens),

            // Is it a block entry?
            [BLOCK_ENTRY, ..] if isWhiteSpaceZ!(~base, 1) =>
            {
                self.block_collection_entry(base, tokens)
            },

            // Is it an explicit key?
            [EXPLICIT_KEY, ..] if self.context.is_flow() || isWhiteSpaceZ!(~base, 1) =>
            {
                self.explicit_key(base, tokens)
            },

            // Is it a value?
            [VALUE, ..] if isWhiteSpaceZ!(~base, 1) || self.context.is_flow() =>
            {
                self.value(base, tokens)
            },

            // Is it an anchor or alias?
            [ANCHOR, ..] | [ALIAS, ..] => self.anchor(base, tokens),

            // Is it a tag?
            [TAG, ..] => self.tag(base, tokens),

            // Is it a block scalar?
            [c @ LITERAL, ..] | [c @ FOLDED, ..] if self.context.is_block() =>
            {
                self.block_scalar(base, tokens, *c == FOLDED)
            },

            // Is it a flow scalar?
            [SINGLE, ..] | [DOUBLE, ..] => self.flow_scalar(base, tokens),

            // Is it a plain scalar?
            _ if self.is_plain_scalar(*base) => self.plain_scalar(base, tokens),

            // Otherwise its an error
            _ => return Err(ScanError::UnknownDelimiter),
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
                // Reset indent to starting level
                self.unroll_indent(tokens, STARTING_INDENT)?;

                // Reset saved key
                self.remove_saved_key()?;

                // Set stream state to finished
                self.state = StreamState::Done;

                enqueue!(Token::StreamEnd, :self.stats => tokens);
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

    fn document_marker(&mut self, buffer: &mut &str, tokens: &mut Tokens, start: bool)
        -> Result<()>
    {
        let token = match start
        {
            true => Token::DocumentStart,
            false => Token::DocumentEnd,
        };

        // Reset indent to starting level
        self.unroll_indent(tokens, STARTING_INDENT)?;

        // Reset saved key
        self.remove_saved_key()?;

        // A key cannot follow a document marker
        self.simple_key_allowed = false;

        advance!(*buffer, :self.stats, 3);

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn directive<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        let mut buffer = *base;
        let mut stats = MStats::new();

        if !check!(~buffer => [DIRECTIVE, ..])
        {
            return Ok(());
        }

        // Reset indent to starting level
        self.unroll_indent(tokens, STARTING_INDENT)?;

        // Reset saved key
        self.remove_saved_key()?;

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

    fn plain_scalar<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        let buffer = *base;
        let mut stats = self.stats.clone();

        self.save_key(!REQUIRED)?;

        let (token, amt) = scan_plain_scalar(buffer, &mut stats, &self.context)?;

        // A simple key cannot follow a plain scalar, there must be
        // an indicator or new line before a key is valid
        // again.
        self.simple_key_allowed = false;

        advance!(*base, amt);
        self.stats = stats;

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn block_scalar<'de>(
        &mut self,
        base: &mut &'de str,
        tokens: &mut Tokens<'de>,
        fold: bool,
    ) -> Result<()>
    {
        let buffer = *base;
        let mut stats = self.stats.clone();

        // Remove any saved keys
        self.remove_saved_key()?;

        // A block scalar cannot be a key, therefore a key may
        // always follow a block scalar.
        self.simple_key_allowed = true;

        let (token, amt) = scan_block_scalar(buffer, &mut stats, &self.context, fold)?;

        advance!(*base, amt);
        self.stats = stats;

        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn explicit_key<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        let block_context = self.context.is_block();
        /*
         * If in the block context we may need to add indentation
         * tokens to the stream, and we need an additional
         * check that keys are currently legal.
         *
         * This can occur, for example if you have the following
         * YAML:
         *
         *      !!str ? 'whoops, tag is': 'in the wrong place'
         *      ^^^^^^^
         *      Invalid token sequence
         *
         * As node decorators (tags, anchors, aliases) must be
         * directly preceding the node
         */
        if block_context
        {
            // Ensure that keys are legal
            if !self.simple_key_allowed
            {
                return Err(ScanError::InvalidKey);
            }

            // Increase the indentation level, and push a
            // BlockMappingStart token to the queue, if
            // required
            roll_indent(
                &mut self.context,
                tokens,
                self.stats.read,
                self.stats.lines,
                self.stats.column,
                BLOCK_MAP,
            )?;
        }

        // Remove any saved implicit key
        self.remove_saved_key()?;

        /* Another key may follow an explicit key in the block
         * context, typically when this explicit key is a
         * mapping node, and the mapping starts inline with the
         * explicit key. E.g:
         *
         *      ? my key: value
         *      : value
         *
         * is equivalent to
         *
         *      ? { my key: value }: value
         */
        self.simple_key_allowed = block_context;

        advance!(*base, :self.stats, 1);

        enqueue!(Token::Key, :self.stats => tokens);

        Ok(())
    }

    /// Fetch a value token (':') from .base, adding to
    /// .tokens. Also handles unwinding any saved
    /// keys and indentation increases, as needed
    fn value<'de>(&mut self, base: &mut &'de str, tokens: &mut Tokens<'de>) -> Result<()>
    {
        // If we found a simple key
        match self.key.saved().take()
        {
            Some(saved) if saved.key().allowed() =>
            {
                let key_stats = saved.stats();

                // Increase the indentation level if required, adding a
                // block mapping start token
                roll_indent(
                    &mut self.context,
                    tokens,
                    key_stats.read,
                    key_stats.lines,
                    key_stats.column,
                    BLOCK_MAP,
                )?;

                // Then push a token to the queue
                enqueue!(Token::Key, :key_stats => tokens);

                // A key cannot follow another key
                self.simple_key_allowed = false;
            },
            // Otherwise we must have found a complex key ('?') previously, or a scalar that is an
            // invalid key
            _ =>
            {
                let block_context = self.context.is_block();

                if block_context
                {
                    // Check if keys are legal
                    if !self.simple_key_allowed
                    {
                        return Err(ScanError::InvalidValue);
                    }

                    // Increase the indentation level if required, adding a
                    // block mapping start token
                    roll_indent(
                        &mut self.context,
                        tokens,
                        self.stats.read,
                        self.stats.lines,
                        self.stats.column,
                        BLOCK_MAP,
                    )?;
                }

                // A simple key is allowed after a value in the block
                // context
                self.simple_key_allowed = block_context;
            },
        }

        advance!(*base, :self.stats, 1);

        enqueue!(Token::Value, :self.stats => tokens);

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
                tokens,
                self.stats.read,
                self.stats.lines,
                self.stats.column,
                !BLOCK_MAP,
            ),
            false => Err(ScanError::InvalidBlockEntry),
        }?;

        // Check if the current block context is zero
        // indented
        let is_zero_indented = self.context.indents().last().map_or(false, |entry| {
            entry.indent() == self.stats.column && entry.line < self.stats.lines
        });

        // If it is, we need to update the line to the
        // current, to disarm pop_zero_indent_sequence
        if is_zero_indented
        {
            let current = self.stats.lines;

            if let Some(entry) = self.context.indents_mut().last_mut()
            {
                entry.line = current;
            }
        }

        // Reset saved key
        self.remove_saved_key()?;

        // A key is possible after a '-'
        self.simple_key_allowed = true;

        advance!(*base, :self.stats, 1);

        let token = Token::BlockEntry;
        enqueue!(token, :self.stats => tokens);

        Ok(())
    }

    fn unroll_indent<'de, T>(&mut self, tokens: &mut Tokens<'de>, column: T) -> Result<()>
    where
        T: Into<Indent>,
    {
        unroll_indent(&mut self.context, self.stats.read, tokens, column)
    }

    /// Check if the current saved key (if it exists) has
    /// expired, removing it if it has
    fn expire_stale_saved_key(&mut self) -> Result<()>
    {
        if let Some(ref mut saved) = self.key.saved()
        {
            let key = saved.key();
            let key_stats = saved.stats();

            /*
             * The YAML spec requires that implicit keys are
             *
             * 1. Limited to a single line
             * 2. Must be less than 1024 characters, including
             *    trailing whitespace to a ': '
             *
             * https://yaml.org/spec/1.2/spec.html#ns-s-implicit-yaml-key(c)
             */
            if key.allowed()
                && (key_stats.lines < self.stats.lines || key_stats.read + 1024 < self.stats.read)
            {
                // If the key was required, it is an error for us not to
                // have found it before the cutoff
                if key.required()
                {
                    return Err(ScanError::MissingValue);
                }

                *saved.key_mut() = KeyPossible::No
            }
        }

        Ok(())
    }

    /// Manages the decrement of zero indented block
    /// sequences
    fn pop_zero_indent_sequence<'de>(
        &mut self,
        base: &'de str,
        tokens: &mut Tokens<'de>,
    ) -> Result<()>
    {
        if let Some(entry) = self.context.indents().last()
        {
            /*
             * Pop an indentation level if, and only if:
             * 1. Current line != entry's line
             * 2. Current indentation is for a sequence
             * 3. The next byte sequence is not a block entry
             * 4. The entry was flagged zero_indented
             */
            if entry.line < self.stats.lines
                && entry.zero_indented
                && entry.kind == Marker::BlockSequenceStart
                && (!check!(~base => b'-'))
            {
                let read = self.stats.read;

                self.context.pop_indent(|_| {
                    enqueue!(Token::BlockEnd, read => tokens);
                    Ok(())
                })?;
            }
        }

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

    /// Checks if .base starts with a character that could
    /// be a plain scalar
    fn is_plain_scalar(&self, base: &str) -> bool
    {
        if isWhiteSpaceZ!(~base)
        {
            return false;
        }

        /*
         * Per the YAML spec, a plain scalar cannot start with
         * any YAML indicators, excluding ':' '?' '-' in
         * certain circumstances.
         *
         * See:
         *      YAML 1.2: Section 7.3.3
         *      yaml.org/spec/1.2/spec.html#ns-plain-first(c)
         */
        match base.as_bytes()
        {
            [DIRECTIVE, ..]
            | [ANCHOR, ..]
            | [ALIAS, ..]
            | [TAG, ..]
            | [SINGLE, ..]
            | [DOUBLE, ..]
            | [FLOW_MAPPING_START, ..]
            | [FLOW_SEQUENCE_START, ..]
            | [FLOW_MAPPING_END, ..]
            | [FLOW_SEQUENCE_END]
            | [FLOW_ENTRY, ..]
            | [LITERAL, ..]
            | [FOLDED, ..]
            | [COMMENT, ..]
            | [RESERVED_1, ..]
            | [RESERVED_2, ..] => false,
            [VALUE, ..] | [EXPLICIT_KEY, ..] | [BLOCK_ENTRY, ..]
                if !is_plain_safe_c(base, 1, self.context.is_block()) =>
            {
                false
            },
            _ => true,
        }
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
            &ALIAS => Self::Alias,
            &ANCHOR => Self::Anchor,
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

/// Roll the indentation level and push a block collection
/// indent token to the indent stack if required
fn roll_indent<'de>(
    context: &mut Context,
    tokens: &mut Tokens<'de>,
    mark: usize,
    line: usize,
    column: usize,
    map: bool,
) -> Result<()>
{
    let token = match map
    {
        true => Token::BlockMappingStart,
        false => Token::BlockSequenceStart,
    };

    if context.is_block()
    {
        // If the indent is greater, we don't need to worry about
        // same level sequences
        if context.indent() < column
        {
            context.indent_increment(column, line, map)?;

            enqueue!(token, mark => tokens);
        }
        // Otherwise we need to check if this is:
        // 1. A sequence
        // 2. At the same indentation level
        // 3. Is the first element of this sequence
        else if (!map) && context.indent() == column
        {
            let add_token = context
                .indents()
                .last()
                .map_or(false, |entry| entry.kind == Marker::BlockMappingStart);

            if add_token
            {
                context.indent_increment(column, line, map)?;

                context.indents_mut().last_mut().unwrap().zero_indented = true;

                enqueue!(token, mark => tokens);
            }
        }
    }

    Ok(())
}

/// Unroll indentation level until we reach .column, pushing
/// a block collection unindent token for every stored
/// indent level
fn unroll_indent<'de, T>(
    context: &mut Context,
    mark: usize,
    tokens: &mut Tokens<'de>,
    column: T,
) -> Result<()>
where
    T: Into<Indent>,
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

/// Checks if the character at .offset is "safe" to start a
/// plain scalar with, as defined in
///
/// yaml.org/spec/1.2/spec.html#ns-plain-safe(c)
fn is_plain_safe_c(base: &str, offset: usize, block_context: bool) -> bool
{
    let flow_context = !block_context;
    let not_flow_indicator = !check!(~base, offset => b',' | b'[' | b']' | b'{' | b'}');

    block_context || (flow_context && not_flow_indicator)
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
const BLOCK_ENTRY: u8 = b'-';
const EXPLICIT_KEY: u8 = b'?';
const LITERAL: u8 = b'|';
const FOLDED: u8 = b'>';
const COMMENT: u8 = b'#';
const RESERVED_1: u8 = b'@';
const RESERVED_2: u8 = b'`';

const COMMENTS: bool = true;
const REQUIRED: bool = true;
const BLOCK_MAP: bool = true;

#[cfg(test)]
mod tests
{
    #[macro_use]
    mod macros;

    mod anchor;
    mod collection;
    mod complex;
    mod directive;
    mod document;
    mod key;
    mod scalar;
    mod tag;
    mod whitespace;

    use super::*;
    use crate::token::{ScalarStyle::*, Token::*};

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
                let skip = match widthOf!(~buffer)
                {
                    0 => unreachable!(),
                    n => n,
                };

                advance!(buffer, :stats, skip);
            }
        }

        stats
    }
}
