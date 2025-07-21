use super::byte_cursor::ByteCursor;
use super::error::ParsiCombError;

/// Core parser trait for parser combinators
pub trait Parser<'code>: Sized {
    type Output;
    
    /// Attempt to parse from the given cursor position
    /// 
    /// Returns Ok with the parsed value and updated cursor on success,
    /// or Err if the parse fails. Failures should not consume input.
    fn parse(&self, cursor: ByteCursor<'code>) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>>;
}