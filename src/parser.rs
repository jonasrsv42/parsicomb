use crate::cursors::Cursor;
use crate::error::ErrorNode;
use std::error::Error;

/// Core parser trait for parser combinators
pub trait Parser<'code> {
    /// The cursor type this parser operates on
    type Cursor: Cursor<'code>;

    /// The output type produced by successful parsing
    type Output;

    /// The error type produced by failed parsing
    type Error: Error + ErrorNode<'code, Element = <Self::Cursor as Cursor<'code>>::Element>;

    /// Attempt to parse from the given cursor position
    ///
    /// Returns Ok with the parsed value and updated cursor on success,
    /// or Err if the parse fails. Failures should not consume input.
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error>;
}
