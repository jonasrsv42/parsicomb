use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use crate::error::ErrorPosition;
use std::fmt;

/// Trait for terminal/leaf error types that can be converted to a base type for comparison
pub trait OrBranch {
    type Base: OrBase;
    fn furthest(self) -> Self::Base;
}

/// Trait for error types that can be directly compared by position
/// This is a marker trait for the "flattened" error types
pub trait OrBase: ErrorPosition {}

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
                write!(
                    f,
                    "Both parsers failed - First: {}, Second: {}",
                    first, second
                )
            }
        }
    }
}

impl<E1, E2> std::error::Error for OrError<E1, E2>
where
    E1: std::error::Error,
    E2: std::error::Error,
{
}

// ParsicombError is both OrBranch (converts to itself) and OrBase (terminal type)
impl<'code> OrBase for crate::ParsicombError<'code> {}

impl<'code> OrBranch for crate::ParsicombError<'code> {
    type Base = Self;

    fn furthest(self) -> Self::Base {
        self // Already the base type
    }
}

// OrError implements OrBranch when both sides are OrBranch with the same Base type
impl<E1, E2> OrBranch for OrError<E1, E2>
where
    E1: OrBranch,
    E2: OrBranch<Base = E1::Base>,
{
    type Base = E1::Base;

    fn furthest(self) -> Self::Base {
        match self {
            OrError::BothFailed { first, second } => {
                let first_base = first.furthest();
                let second_base = second.furthest();

                if first_base.byte_position() >= second_base.byte_position() {
                    first_base
                } else {
                    second_base
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
            Err(first_error) => match self.parser2.parse(cursor) {
                Ok(result) => Ok(result),
                Err(second_error) => Err(OrError::BothFailed {
                    first: first_error,
                    second: second_error,
                }),
            },
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

    #[test]
    fn test_or_error_furthest_simple() {
        use crate::error::{CodeLoc, ErrorPosition, ParsicombError};

        let data = b"xyz";
        let error1 = ParsicombError::SyntaxError {
            message: "first error".into(),
            loc: CodeLoc::new(data, 0), // position 0
        };
        let error2 = ParsicombError::SyntaxError {
            message: "second error".into(),
            loc: CodeLoc::new(data, 2), // position 2 (further)
        };

        let or_error = OrError::BothFailed {
            first: error1,
            second: error2,
        };
        let furthest = or_error.furthest();

        assert_eq!(furthest.byte_position(), 2);
        assert!(furthest.to_string().contains("second error"));
    }

    #[test]
    fn test_or_error_furthest_first_wins() {
        use crate::error::{CodeLoc, ErrorPosition, ParsicombError};

        let data = b"xyz";
        let error1 = ParsicombError::SyntaxError {
            message: "first error".into(),
            loc: CodeLoc::new(data, 3), // position 3 (further)
        };
        let error2 = ParsicombError::SyntaxError {
            message: "second error".into(),
            loc: CodeLoc::new(data, 1), // position 1
        };

        let or_error = OrError::BothFailed {
            first: error1,
            second: error2,
        };
        let furthest = or_error.furthest();

        assert_eq!(furthest.byte_position(), 3);
        assert!(furthest.to_string().contains("first error"));
    }

    #[test]
    fn test_or_error_auto_recursive_furthest() {
        use crate::error::{CodeLoc, ErrorPosition, ParsicombError};

        let data = b"abcdefghij";

        // Create deeply nested structure: OrError<OrError<E1, E2>, E3>
        let error1 = ParsicombError::SyntaxError {
            message: "error at pos 1".into(),
            loc: CodeLoc::new(data, 1),
        };
        let error2 = ParsicombError::SyntaxError {
            message: "error at pos 8".into(), // This should be furthest
            loc: CodeLoc::new(data, 8),
        };
        let error3 = ParsicombError::SyntaxError {
            message: "error at pos 5".into(),
            loc: CodeLoc::new(data, 5),
        };

        // Build the nested structure
        let inner_or = OrError::BothFailed {
            first: error1,
            second: error2,
        };
        let outer_or = OrError::BothFailed {
            first: inner_or,
            second: error3,
        };

        // Use the new OrBranch system - this automatically handles recursion!
        let furthest = outer_or.furthest();

        assert_eq!(furthest.byte_position(), 8);
        assert!(furthest.to_string().contains("error at pos 8"));
    }
}
