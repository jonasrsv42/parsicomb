//! A parser followed by a suffix, keeping only the parser's output

use crate::and::AndError;
use crate::parser::Parser;

/// Parser combinator that matches `parser` then `suffix`, and returns only
/// `parser`'s output
///
/// ```
/// use parsicomb::{ByteCursor, Parser, TerminatedExt};
/// use parsicomb::ascii::u64;
/// use parsicomb::byte::is_byte;
///
/// let statement = u64().terminated_by(is_byte(b';'));
/// assert_eq!(statement.parse(ByteCursor::new(b"42;")).unwrap().0, 42);
/// ```
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Terminated<P, PSuffix> {
    parser: P,
    suffix: PSuffix,
}

impl<P, PSuffix> Terminated<P, PSuffix> {
    /// Same as [`terminated`]
    pub fn new(parser: P, suffix: PSuffix) -> Self {
        Terminated { parser, suffix }
    }
}

impl<P, PSuffix> Parser for Terminated<P, PSuffix>
where
    P: Parser,
    PSuffix: Parser<Cursor = P::Cursor>,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = AndError<P::Error, PSuffix::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (output, cursor) = self.parser.parse(cursor).map_err(AndError::FirstParser)?;
        let (_, cursor) = self.suffix.parse(cursor).map_err(AndError::SecondParser)?;
        Ok((output, cursor))
    }
}

/// Create a parser that matches `parser` then `suffix`, keeping `parser`'s
/// output
pub fn terminated<P, PSuffix>(parser: P, suffix: PSuffix) -> Terminated<P, PSuffix>
where
    P: Parser,
    PSuffix: Parser<Cursor = P::Cursor>,
{
    Terminated::new(parser, suffix)
}

/// Extension trait to add .terminated_by() method support for parsers
pub trait TerminatedExt: Parser + Sized {
    /// Match `suffix` after this parser, keeping only this parser's output
    fn terminated_by<PSuffix>(self, suffix: PSuffix) -> Terminated<Self, PSuffix>
    where
        PSuffix: Parser<Cursor = Self::Cursor>,
    {
        Terminated::new(self, suffix)
    }
}

impl<P: Parser> TerminatedExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte::is_byte;
    use crate::{ByteCursor, Cursor};

    #[test]
    fn test_keeps_the_parser_output() {
        let parser = terminated(is_byte(b'x'), is_byte(b';'));
        let (value, cursor) = parser.parse(ByteCursor::new(b"x;!")).unwrap();
        assert_eq!((value, cursor.position()), (b'x', 2));
        assert!(parser.parse(ByteCursor::new(b"x!")).is_err());
    }
}
