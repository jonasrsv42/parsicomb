use super::parser::Parser;
use crate::error::{ErrorLeaf, ErrorNode};
use std::fmt;

/// Error type for ThenOptionally parser that can wrap errors from the first parser
/// The second parser is optional, so it doesn't contribute to errors
#[derive(Debug)]
pub enum ThenOptionallyError<E> {
    /// Error from the first parser (which is required)
    FirstParser(E),
}

impl<E: fmt::Display> fmt::Display for ThenOptionallyError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThenOptionallyError::FirstParser(e) => write!(f, "Required parser failed: {}", e),
        }
    }
}

impl<E> std::error::Error for ThenOptionallyError<E> where E: std::error::Error {}

// Implement From<ThenOptionallyError<E>> for ParsicombError where E can convert to ParsicombError
impl<'code, E> From<ThenOptionallyError<E>> for crate::ParsicombError<'code>
where
    E: Into<crate::ParsicombError<'code>>,
{
    fn from(err: ThenOptionallyError<E>) -> crate::ParsicombError<'code> {
        match err {
            ThenOptionallyError::FirstParser(e) => e.into(),
        }
    }
}

// Implement ErrorNode for ThenOptionallyError to enable furthest-error selection
impl<'code, E> ErrorNode<'code> for ThenOptionallyError<E>
where
    E: ErrorNode<'code>,
{
    fn likely_error(self) -> Box<dyn ErrorLeaf + 'code> {
        match self {
            // First parser failed - return its error
            ThenOptionallyError::FirstParser(e) => e.likely_error(),
        }
    }
}

/// Parser combinator that sequences two parsers where the second is optional
///
/// Returns a tuple with the first parser's result and an Option containing the second
/// parser's result (Some if it succeeded, None if it failed).
///
/// Example:
/// ```
/// use parsicomb::ascii::i64;
/// use parsicomb::byte::is_byte;
/// use parsicomb::byte_cursor::ByteCursor;
/// use parsicomb::then_optionally::ThenOptionallyExt;
/// use parsicomb::parser::Parser;
///
/// let data = b"123.456";
/// let cursor = ByteCursor::new(data);
/// let (result, cursor) = i64()
///     .then_optionally(is_byte(b'.'))
///     .parse(cursor).unwrap();
/// assert_eq!(result.0, 123);
/// assert_eq!(result.1, Some(b'.'));
///
/// let data = b"123xyz";
/// let cursor = ByteCursor::new(data);
/// let (result, cursor) = i64()
///     .then_optionally(is_byte(b'.'))
///     .parse(cursor).unwrap();
/// assert_eq!(result.0, 123);
/// assert_eq!(result.1, None);
/// ```
pub struct ThenOptionally<P1, P2> {
    parser1: P1,
    parser2: P2,
}

impl<P1, P2> ThenOptionally<P1, P2> {
    pub fn new(parser1: P1, parser2: P2) -> Self {
        ThenOptionally { parser1, parser2 }
    }
}

impl<'code, P1, P2> Parser<'code> for ThenOptionally<P1, P2>
where
    P1: Parser<'code>,
    P2: Parser<'code, Cursor = P1::Cursor>,
{
    type Cursor = P1::Cursor;
    type Output = (P1::Output, Option<P2::Output>);
    type Error = ThenOptionallyError<P1::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (result1, cursor) = self
            .parser1
            .parse(cursor)
            .map_err(ThenOptionallyError::FirstParser)?;

        // Try the second parser, but don't fail if it doesn't succeed
        match self.parser2.parse(cursor) {
            Ok((result2, cursor)) => Ok(((result1, Some(result2)), cursor)),
            Err(_) => Ok(((result1, None), cursor)),
        }
    }
}

/// Convenience function to create a ThenOptionally parser
pub fn then_optionally<'code, P1, P2>(parser1: P1, parser2: P2) -> ThenOptionally<P1, P2>
where
    P1: Parser<'code>,
    P2: Parser<'code>,
{
    ThenOptionally::new(parser1, parser2)
}

/// Extension trait to add .then_optionally() method support for parsers
pub trait ThenOptionallyExt<'code>: Parser<'code> + Sized {
    fn then_optionally<P>(self, other: P) -> ThenOptionally<Self, P>
    where
        P: Parser<'code>,
    {
        ThenOptionally::new(self, other)
    }
}

/// Implement ThenOptionallyExt for all parsers
impl<'code, P> ThenOptionallyExt<'code> for P where P: Parser<'code> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::Parser;
    use crate::ascii::i64;
    use crate::byte::is_byte;

    #[test]
    fn test_then_optionally_both_succeed() {
        let data = b"123.456";
        let cursor = ByteCursor::new(data);
        let parser = i64().then_optionally(is_byte(b'.'));

        let ((number, dot), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(number, 123);
        assert_eq!(dot, Some(b'.'));
        assert_eq!(cursor.value().unwrap(), b'4');
    }

    #[test]
    fn test_then_optionally_first_succeeds_second_fails() {
        let data = b"123xyz";
        let cursor = ByteCursor::new(data);
        let parser = i64().then_optionally(is_byte(b'.'));

        let ((number, dot), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(number, 123);
        assert_eq!(dot, None);
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_then_optionally_first_fails() {
        let data = b"xyz123";
        let cursor = ByteCursor::new(data);
        let parser = i64().then_optionally(is_byte(b'.'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_then_optionally_method_syntax() {
        let data = b"A5";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').then_optionally(is_byte(b'5'));

        let ((a, five), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(a, b'A');
        assert_eq!(five, Some(b'5'));
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_then_optionally_method_syntax_second_fails() {
        let data = b"Ax";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').then_optionally(is_byte(b'5'));

        let ((a, five), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(a, b'A');
        assert_eq!(five, None);
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_then_optionally_function_syntax() {
        let data = b"XY";
        let cursor = ByteCursor::new(data);
        let parser = then_optionally(is_byte(b'X'), is_byte(b'Y'));

        let ((x, y), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(x, b'X');
        assert_eq!(y, Some(b'Y'));
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_then_optionally_chain() {
        let data = b"ABC";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A')
            .then_optionally(is_byte(b'B'))
            .then_optionally(is_byte(b'C'));

        let (((a, b), c), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(a, b'A');
        assert_eq!(b, Some(b'B'));
        assert_eq!(c, Some(b'C'));
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_then_optionally_chain_partial_success() {
        let data = b"ABX";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A')
            .then_optionally(is_byte(b'B'))
            .then_optionally(is_byte(b'C'));

        let (((a, b), c), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(a, b'A');
        assert_eq!(b, Some(b'B'));
        assert_eq!(c, None);
        assert_eq!(cursor.value().unwrap(), b'X');
    }

    #[test]
    fn test_then_optionally_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').then_optionally(is_byte(b'B'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }
}
