use super::parser::Parser;
use crate::error::{ErrorLeaf, ErrorNode};
use std::fmt;

/// Error type for And parser, holding the error of whichever parser failed
#[derive(Debug)]
pub enum AndError<Err1, Err2> {
    /// Error from the first parser
    FirstParser(Err1),
    /// Error from the second parser
    SecondParser(Err2),
}

impl<Err1: fmt::Display, Err2: fmt::Display> fmt::Display for AndError<Err1, Err2> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AndError::FirstParser(e) => write!(f, "First parser failed: {}", e),
            AndError::SecondParser(e) => write!(f, "Second parser failed: {}", e),
        }
    }
}

impl<Err1, Err2> std::error::Error for AndError<Err1, Err2>
where
    Err1: std::error::Error,
    Err2: std::error::Error,
{
}

impl<Err1, Err2> ErrorNode for AndError<Err1, Err2>
where
    Err1: ErrorNode,
    Err2: ErrorNode<Element = Err1::Element>,
{
    type Element = Err1::Element;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        match self {
            AndError::FirstParser(e) => e.likely_error(),
            AndError::SecondParser(e) => e.likely_error(),
        }
    }
}

/// Parser combinator that sequences two parsers and returns both results as a tuple
///
/// Note: When chaining multiple `.and()` calls, this produces nested tuples like
/// `(((a, b), c), d)` rather than flat tuples like `(a, b, c, d)`. This is due
/// to Rust's lack of variadic generics. While we could use macros to work around
/// this for specific arities, the nested tuple approach is more general and the
/// destructuring pattern is explicit about the parsing order.
///
/// Example:
/// ```
/// use parsicomb::ascii::{i64, u64};
/// use parsicomb::byte::is_byte;
/// use parsicomb::ByteCursor;
/// use parsicomb::and::AndExt;
/// use parsicomb::parser::Parser;
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
pub struct And<P1, P2> {
    parser1: P1,
    parser2: P2,
}

impl<P1, P2> And<P1, P2> {
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

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (out1, cursor) = self.parser1.parse(cursor).map_err(AndError::FirstParser)?;
        let (out2, cursor) = self.parser2.parse(cursor).map_err(AndError::SecondParser)?;
        Ok(((out1, out2), cursor))
    }
}

/// Convenience function to create an And parser
pub fn and<P1, P2>(parser1: P1, parser2: P2) -> And<P1, P2>
where
    P1: Parser,
    P2: Parser<Cursor = P1::Cursor>,
{
    And::new(parser1, parser2)
}

/// Extension trait to add .and() method support for parsers
pub trait AndExt: Parser + Sized {
    fn and<P>(self, other: P) -> And<Self, P>
    where
        P: Parser<Cursor = Self::Cursor>,
    {
        And::new(self, other)
    }
}

/// Implement AndExt for all parsers
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
