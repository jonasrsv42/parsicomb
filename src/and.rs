use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use crate::ParsiCombError;

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
/// let cursor = ByteCursor::new(data).unwrap();
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
    
    fn parse(&self, cursor: ByteCursor<'code>) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>> {
        let (result1, cursor) = self.parser1.parse(cursor)?;
        let (result2, cursor) = self.parser2.parse(cursor)?;
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
    use crate::byte::is_byte;
    use crate::ascii::i64;

    #[test]
    fn test_and_both_succeed() {
        let data = b"A5xyz";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = is_byte(b'A').and(is_byte(b'5'));
        
        let ((byte1, byte2), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte1, b'A');
        assert_eq!(byte2, b'5');
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_and_first_fails() {
        let data = b"Bxyz";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = is_byte(b'A').and(is_byte(b'x'));
        
        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_and_second_fails() {
        let data = b"Axyz";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = is_byte(b'A').and(is_byte(b'5'));
        
        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_and_method_syntax() {
        let data = b"123.";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = i64().and(is_byte(b'.'));
        
        let ((number, dot), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(number, 123);
        assert_eq!(dot, b'.');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_and_chain() {
        let data = b"A5B";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = is_byte(b'A')
            .and(is_byte(b'5'))
            .and(is_byte(b'B'));
        
        let (((a, five), b), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(a, b'A');
        assert_eq!(five, b'5');
        assert_eq!(b, b'B');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_and_function_syntax() {
        let data = b"XY";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = and(is_byte(b'X'), is_byte(b'Y'));
        
        let ((x, y), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(x, b'X');
        assert_eq!(y, b'Y');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
