use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use areamy::error::Error;

/// Parser combinator that tries the first parser, and if it fails, tries the second parser
pub struct Or<P1, P2> {
    parser1: P1,
    parser2: P2,
}

impl<P1, P2> Or<P1, P2> {
    pub fn new(parser1: P1, parser2: P2) -> Self {
        Or { parser1, parser2 }
    }
}

impl<'a, P1, P2, O> Parser<'a> for Or<P1, P2>
where
    P1: Parser<'a, Output = O>,
    P2: Parser<'a, Output = O>,
{
    type Output = O;
    
    fn parse(&self, cursor: ByteCursor<'a>) -> Result<(Self::Output, ByteCursor<'a>), Error> {
        match self.parser1.parse(cursor) {
            Ok(result) => Ok(result),
            Err(_) => self.parser2.parse(cursor),
        }
    }
}

/// Extension trait to add .or() method support for parsers
pub trait OrExt<'a>: Parser<'a> + Sized {
    fn or<P>(self, other: P) -> Or<Self, P>
    where
        P: Parser<'a, Output = Self::Output>,
    {
        Or::new(self, other)
    }
}

/// Implement OrExt for all parsers
impl<'a, P> OrExt<'a> for P where P: Parser<'a> {}

/// Convenience function to create an Or parser
pub fn or<'a, P1, P2, O>(parser1: P1, parser2: P2) -> Or<P1, P2>
where
    P1: Parser<'a, Output = O>,
    P2: Parser<'a, Output = O>,
{
    Or::new(parser1, parser2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte::is_byte;

    #[test]
    fn test_or_first_succeeds() {
        let data = b"abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = or(
            is_byte(b'a'),
            is_byte(b'b')
        );
        
        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'a');
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_or_second_succeeds() {
        let data = b"bcd";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = or(
            is_byte(b'a'),
            is_byte(b'b')
        );
        
        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'b');
        assert_eq!(cursor.value().unwrap(), b'c');
    }

    #[test]
    fn test_or_both_fail() {
        let data = b"xyz";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = or(
            is_byte(b'a'),
            is_byte(b'b')
        );
        
        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_or_method_syntax() {
        let data = b"b";
        let cursor = ByteCursor::new(data).unwrap();
        
        // Using .or() method
        let parser = is_byte(b'a').or(is_byte(b'b'));
        
        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'b');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_or_method_chain() {
        let data = b"c";
        let cursor = ByteCursor::new(data).unwrap();
        
        // Chaining with .or() method
        let parser = is_byte(b'a')
            .or(is_byte(b'b'))
            .or(is_byte(b'c'));
        
        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'c');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_or_method_complex_chain() {
        let data = b"d";
        let cursor = ByteCursor::new(data).unwrap();
        
        // Complex chain with .or() method
        let parser = is_byte(b'a')
            .or(is_byte(b'b'))
            .or(is_byte(b'c'))
            .or(is_byte(b'd'));
        
        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'd');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
