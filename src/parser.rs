use crate::cursor::Cursor;
use crate::error::ErrorNode;

/// Core parser trait for parser combinators
pub trait Parser {
    /// The cursor type this parser operates on
    type Cursor: Cursor;

    /// The output type produced by successful parsing
    type Output;

    /// The error type produced by failed parsing
    type Error: ErrorNode<Element = <Self::Cursor as Cursor>::Element>;

    /// Attempt to parse from the given cursor position
    ///
    /// Returns Ok with the parsed value and updated cursor on success,
    /// or Err if the parse fails; on failure the caller keeps its original cursor.
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error>;
}
