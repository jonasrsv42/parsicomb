use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use std::fmt;

/// Error type for And parser that can wrap errors from either the first or second parser
#[derive(Debug)]
pub enum AndError<E1, E2> {
    /// Error from the first parser
    FirstParser(E1),
    /// Error from the second parser
    SecondParser(E2),
}

impl<E> AndError<E, E> {
    /// Extract the inner error when both error types are the same
    pub fn into_inner(self) -> E {
        match self {
            AndError::FirstParser(e) => e,
            AndError::SecondParser(e) => e,
        }
    }
}

impl<E1: fmt::Display, E2: fmt::Display> fmt::Display for AndError<E1, E2> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AndError::FirstParser(e) => write!(f, "First parser failed: {}", e),
            AndError::SecondParser(e) => write!(f, "Second parser failed: {}", e),
        }
    }
}

impl<E1, E2> std::error::Error for AndError<E1, E2>
where
    E1: std::error::Error,
    E2: std::error::Error,
{
}

// Implement From<AndError<E1, E2>> for ParsicombError where both E1 and E2 can convert to ParsicombError
impl<'code, E1, E2> From<AndError<E1, E2>> for crate::ParsicombError<'code>
where
    E1: Into<crate::ParsicombError<'code>>,
    E2: Into<crate::ParsicombError<'code>>,
{
    fn from(err: AndError<E1, E2>) -> crate::ParsicombError<'code> {
        match err {
            AndError::FirstParser(e1) => e1.into(),
            AndError::SecondParser(e2) => e2.into(),
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
/// use parsicomb::byte_cursor::ByteCursor;
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

impl<'code, P1, P2> Parser<'code> for And<P1, P2>
where
    P1: Parser<'code>,
    P2: Parser<'code>,
{
    type Output = (P1::Output, P2::Output);
    type Error = AndError<P1::Error, P2::Error>;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
        let (result1, cursor) = self.parser1.parse(cursor).map_err(AndError::FirstParser)?;
        let (result2, cursor) = self.parser2.parse(cursor).map_err(AndError::SecondParser)?;
        Ok(((result1, result2), cursor))
    }
}

/// Convenience function to create an And parser
pub fn and<'code, P1, P2>(parser1: P1, parser2: P2) -> And<P1, P2>
where
    P1: Parser<'code>,
    P2: Parser<'code>,
{
    And::new(parser1, parser2)
}

/// Extension trait to add .and() method support for parsers
pub trait AndExt<'code>: Parser<'code> + Sized {
    fn and<P>(self, other: P) -> And<Self, P>
    where
        P: Parser<'code>,
    {
        And::new(self, other)
    }
}

/// Implement AndExt for all parsers
impl<'code, P> AndExt<'code> for P where P: Parser<'code> {}

#[cfg(test)]
mod tests {
    use super::*;
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
