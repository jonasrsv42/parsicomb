use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use std::fmt;

/// Error type for Or parser that can wrap errors from both parsers when both fail
#[derive(Debug)]
pub enum OrError<E1, E2> {
    /// Both parsers failed
    BothFailed { first: E1, second: E2 },
}

impl<E1: fmt::Display, E2: fmt::Display> fmt::Display for OrError<E1, E2> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrError::BothFailed { first, second } => {
                write!(f, "Both parsers failed - First: {}, Second: {}", first, second)
            }
        }
    }
}

impl<E1, E2> std::error::Error for OrError<E1, E2> 
where
    E1: std::error::Error,
    E2: std::error::Error,
{}

impl<E> OrError<E, E> 
where 
    E: crate::error::ErrorPosition,
{
    /// Returns the error that progressed furthest in the input when both errors are the same type
    pub fn furthest(self) -> E {
        match self {
            OrError::BothFailed { first, second } => {
                if first.byte_position() >= second.byte_position() {
                    first
                } else {
                    second
                }
            }
        }
    }
}

impl<E1, E2> OrError<E1, E2> 
where 
    E1: crate::error::ErrorPosition,
    E2: crate::error::ErrorPosition,
{
    /// Select the furthest error and convert it using the provided functions
    /// This enables handling nested OrError types and different error types
    pub fn select_furthest<F1, F2, T>(self, convert_first: F1, convert_second: F2) -> T 
    where
        F1: FnOnce(E1) -> T,
        F2: FnOnce(E2) -> T,
    {
        match self {
            OrError::BothFailed { first, second } => {
                if first.byte_position() >= second.byte_position() {
                    convert_first(first)
                } else {
                    convert_second(second)
                }
            }
        }
    }
}

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

impl<'code, P1, P2, O> Parser<'code> for Or<P1, P2>
where
    P1: Parser<'code, Output = O>,
    P2: Parser<'code, Output = O>,
{
    type Output = O;
    type Error = OrError<P1::Error, P2::Error>;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
        match self.parser1.parse(cursor) {
            Ok(result) => Ok(result),
            Err(first_error) => {
                match self.parser2.parse(cursor) {
                    Ok(result) => Ok(result),
                    Err(second_error) => Err(OrError::BothFailed { 
                        first: first_error, 
                        second: second_error 
                    }),
                }
            }
        }
    }
}

/// Extension trait to add .or() method support for parsers
pub trait OrExt<'code>: Parser<'code> + Sized {
    fn or<P>(self, other: P) -> Or<Self, P>
    where
        P: Parser<'code, Output = Self::Output>,
    {
        Or::new(self, other)
    }
}

/// Implement OrExt for all parsers
impl<'code, P> OrExt<'code> for P where P: Parser<'code> {}

/// Convenience function to create an Or parser
pub fn or<'code, P1, P2, O>(parser1: P1, parser2: P2) -> Or<P1, P2>
where
    P1: Parser<'code, Output = O>,
    P2: Parser<'code, Output = O>,
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
        let cursor = ByteCursor::new(data);
        let parser = or(is_byte(b'a'), is_byte(b'b'));

        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'a');
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_or_second_succeeds() {
        let data = b"bcd";
        let cursor = ByteCursor::new(data);
        let parser = or(is_byte(b'a'), is_byte(b'b'));

        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'b');
        assert_eq!(cursor.value().unwrap(), b'c');
    }

    #[test]
    fn test_or_both_fail() {
        let data = b"xyz";
        let cursor = ByteCursor::new(data);
        let parser = or(is_byte(b'a'), is_byte(b'b'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_or_method_syntax() {
        let data = b"b";
        let cursor = ByteCursor::new(data);

        // Using .or() method
        let parser = is_byte(b'a').or(is_byte(b'b'));

        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'b');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_or_method_chain() {
        let data = b"c";
        let cursor = ByteCursor::new(data);

        // Chaining with .or() method
        let parser = is_byte(b'a').or(is_byte(b'b')).or(is_byte(b'c'));

        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'c');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_or_method_complex_chain() {
        let data = b"d";
        let cursor = ByteCursor::new(data);

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
