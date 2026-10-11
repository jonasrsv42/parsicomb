//! The [`Parser`] trait

use crate::cursor::Cursor;
use crate::error::ErrorNode;

/// Core parser trait for parser combinators
pub trait Parser {
    /// The cursor type this parser operates on
    type Cursor: Cursor;

    /// The output type produced by successful parsing
    type Output;

    /// The error type produced by failed parsing
    ///
    /// Combinators use the narrowest type that holds every failure they
    /// produce: the cursor's error when they only fail on their own (`not`)
    /// or never fail (`many`, `optional`), the inner parser's error when they
    /// only pass it on (`some`, `separated_list`), and an enum of both when
    /// they do both (`filter`, `all`, `max_depth`).
    type Error: ErrorNode<Element = <Self::Cursor as Cursor>::Element>;

    /// Attempt to parse from the given cursor position
    ///
    /// Returns Ok with the parsed value and updated cursor on success,
    /// or Err if the parse fails; on failure the caller keeps its original cursor.
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error>;
}
