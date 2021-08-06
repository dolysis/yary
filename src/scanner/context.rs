use std::ops::Add;

use crate::scanner::error::{ScanError, ScanResult as Result};

pub(in crate::scanner) const STARTING_INDENT: Indent = Indent(None);

/// Manages the the current YAML context. Contexts are
/// mutually exclusive, that is, you cannot be
/// in both a Flow and Block context simultaneously.
/// Furthermore, it is possible to have deeper
/// levels of Flow nested inside of Flow or Block contexts,
/// but you cannot have a Block context nested inside a Flow
/// context, and this structure will ignore attempts to
/// start a Block context while inside a Flow context.
#[derive(Debug, Clone, Default)]
pub(in crate::scanner) struct Context
{
    // Flow context fields
    flow: usize,

    // Block context fields
    indent:  usize,
    indents: Vec<IndentEntry>,
    started: bool,
}

impl Context
{
    const MAX_RESERVE_AFTER_RESET: usize = 64;

    /// Instantiate a new Context
    pub fn new() -> Self
    {
        Self {
            flow:    0,
            indent:  0,
            indents: Vec::new(),
            started: false,
        }
    }

    /// Reset Context to starting state, typically this
    /// should be used when moving documents
    pub fn reset(&mut self)
    {
        self.flow = 0;
        self.indent = 0;
        self.indents.truncate(Self::MAX_RESERVE_AFTER_RESET);
        self.started = false;
    }

    /// Get the current flow level
    pub fn flow(&self) -> usize
    {
        self.flow
    }

    /// Check if we are currently in the flow context
    pub fn is_flow(&self) -> bool
    {
        self.flow != 0
    }

    pub fn flow_increment(&mut self) -> Result<usize>
    {
        let new = self.flow.checked_add(1).ok_or(ScanError::IntOverflow)?;
        self.flow = new;

        Ok(new)
    }

    pub fn flow_decrement(&mut self) -> Result<usize>
    {
        let new = self.flow.checked_sub(1).ok_or(ScanError::IntOverflow)?;
        self.flow = new;

        Ok(new)
    }

    /// Get the current indent level
    pub fn indent(&self) -> Indent
    {
        match self.started
        {
            true => self.indent.into(),
            false => None.into(),
        }
    }

    pub fn indents(&self) -> &[IndentEntry]
    {
        &self.indents
    }

    pub fn indents_mut(&mut self) -> &mut Vec<IndentEntry>
    {
        &mut self.indents
    }

    /// Check if we are currently in the block context
    pub fn is_block(&self) -> bool
    {
        !self.is_flow()
    }

    /// Increment the current indent level, if not in the
    /// flow context, returning the current level. Note
    /// that this function will only increment the indent if
    /// .column > current_indent and .is_block returns true
    pub fn indent_increment(&mut self, column: usize) -> Result<Indent>
    {
        self.started = true;
        self.indents.push(IndentEntry::from(self.indent));

        self.indent = column;

        Ok(self.indent.into())
    }

    /// Decrement the indent level calling .f for every
    /// level until .column > current_indent,
    /// returning the number of levels decremented
    pub fn indent_decrement<T, F>(&mut self, column: T, mut f: F) -> Result<usize>
    where
        T: Into<Indent>,
        F: FnMut(usize) -> Result<()>,
    {
        let column: Indent = column.into();
        let old = self.indents.len();

        if self.is_block()
        {
            while Indent(Some(self.indent)) > column
            {
                match self.indents.pop()
                {
                    Some(entry) =>
                    {
                        self.indent = entry.indent;

                        f(self.indent)?;
                    },
                    None => break,
                }
            }
        }

        Ok(old - self.indents.len())
    }
}

/// Stack entry for tracking indentation levels, and
/// associated metadata
#[derive(Debug, Clone, Copy)]
pub(in crate::scanner) struct IndentEntry
{
    indent: usize,

    /// Set if the associated indentation is a sequence, and
    /// the scanner has produced a BlockSequenceStart
    /// token for it
    ///
    /// This field is only used to track whether a token has
    /// already been produced for a given list, in cases
    /// where the list is zero indented
    pub sequence_started: bool,
}

impl IndentEntry
{
    pub fn new(indent: usize) -> Self
    {
        Self {
            indent,
            sequence_started: false,
        }
    }

    pub fn indent(&self) -> Indent
    {
        self.indent.into()
    }
}

impl From<usize> for IndentEntry
{
    fn from(indent: usize) -> Self
    {
        Self::new(indent)
    }
}

/// A wrapper around usize, that allows it us to express the
/// "-1"nth indent without needing to use a signed type.
/// This occurs when we have not yet encountered the first
/// map node, and thus the entire document could be a scalar
/// (or sequence!), in which case we don't really have an
/// indent so to speak, hence the "-1"nth-ness
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::scanner) struct Indent(Option<usize>);

impl From<usize> for Indent
{
    fn from(indent: usize) -> Self
    {
        Self(Some(indent))
    }
}

impl From<Option<usize>> for Indent
{
    fn from(maybe: Option<usize>) -> Self
    {
        Self(maybe)
    }
}

impl PartialEq<usize> for Indent
{
    fn eq(&self, other: &usize) -> bool
    {
        match self.0
        {
            Some(ref indent) => indent == other,
            None => false,
        }
    }
}

impl PartialOrd<usize> for Indent
{
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering>
    {
        match self.0
        {
            Some(indent) => indent.partial_cmp(other),
            None => Some(std::cmp::Ordering::Less),
        }
    }
}

impl Add<usize> for Indent
{
    type Output = usize;

    fn add(self, rhs: usize) -> Self::Output
    {
        match self.0
        {
            Some(indent) => indent + rhs,
            None => rhs,
        }
    }
}
