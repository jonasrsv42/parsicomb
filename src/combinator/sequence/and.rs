//! Sequencing: `a.and(b)` runs `a` then `b`

use crate::parser::Parser;

use crate::error::composite_error;

composite_error! {
    /// Error type for And parser, holding the error of whichever parser failed
    pub enum AndError<Err1, Err2> {
        /// Error from the first parser
        FirstParser(Err1),
        /// Error from the second parser
        SecondParser(Err2),
    }
}

/// Parser combinator that sequences two parsers and returns both results as a tuple
///
/// Chained `.and()` calls nest their outputs: `a.and(b).and(c)` outputs
/// `((a, b), c)`.
///
/// Example:
/// ```
/// use parsicomb::prelude::*;
/// use parsicomb::ascii::{i64, u64};
/// use parsicomb::byte::is_byte;
///
/// let data = b"123.456";
/// let cursor = ByteCursor::new(data);
/// let (((int_part, _), frac_part), cursor) = i64()
///     .and(is_byte(b'.'))
///     .and(u64())
///     .parse(cursor).unwrap();
/// assert_eq!(int_part, 123);
/// assert_eq!(frac_part, 456);
/// ```
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct And<P1, P2> {
    parser1: P1,
    parser2: P2,
}

impl<P1, P2> And<P1, P2> {
    /// Same as [`and`]
    pub fn new(parser1: P1, parser2: P2) -> Self {
        And { parser1, parser2 }
    }
}

impl<P1, P2> Parser for And<P1, P2>
where
    P1: Parser,
    P2: Parser<Cursor = P1::Cursor>,
{
    type Cursor = P1::Cursor;
    type Output = (P1::Output, P2::Output);
    type Error = AndError<P1::Error, P2::Error>;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (out1, cursor) = self.parser1.parse(cursor).map_err(AndError::FirstParser)?;
        let (out2, cursor) = self.parser2.parse(cursor).map_err(AndError::SecondParser)?;
        Ok(((out1, out2), cursor))
    }
}

/// Create a parser that runs `parser1`, then `parser2`; see [`And`]
pub fn and<P1, P2>(parser1: P1, parser2: P2) -> And<P1, P2>
where
    P1: Parser,
    P2: Parser<Cursor = P1::Cursor>,
{
    And::new(parser1, parser2)
}

/// Extension trait to add .and() method support for parsers
pub trait AndExt: Parser + Sized {
    /// Run `other` after `self`, outputting both results as a tuple
    fn and<P>(self, other: P) -> And<Self, P>
    where
        P: Parser<Cursor = Self::Cursor>,
    {
        And::new(self, other)
    }
}

impl<P: Parser> AndExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::ascii::i64;
    use crate::byte::is_byte;

    #[test]
    fn test_and_both_succeed() {
        let data = b"A5xyz";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').and(is_byte(b'5'));

        let ((byte1, byte2), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte1, b'A');
        assert_eq!(byte2, b'5');
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_and_first_fails() {
        let data = b"Bxyz";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').and(is_byte(b'x'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_and_second_fails() {
        let data = b"Axyz";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').and(is_byte(b'5'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_and_method_syntax() {
        let data = b"123.";
        let cursor = ByteCursor::new(data);
        let parser = i64().and(is_byte(b'.'));

        let ((number, dot), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(number, 123);
        assert_eq!(dot, b'.');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_and_chain() {
        let data = b"A5B";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').and(is_byte(b'5')).and(is_byte(b'B'));

        let (((a, five), b), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(a, b'A');
        assert_eq!(five, b'5');
        assert_eq!(b, b'B');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_and_function_syntax() {
        let data = b"XY";
        let cursor = ByteCursor::new(data);
        let parser = and(is_byte(b'X'), is_byte(b'Y'));

        let ((x, y), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(x, b'X');
        assert_eq!(y, b'Y');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
