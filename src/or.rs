use super::parser::Parser;
use crate::error::{ErrorLeaf, ErrorNode};
use std::fmt;

/// Error type for Or parser, holding the errors of both alternatives
#[derive(Debug)]
pub enum OrError<Err1, Err2> {
    /// Both parsers failed
    BothFailed { first: Err1, second: Err2 },
}

/// Displays the alternative that got furthest into the input
impl<Err1, Err2> fmt::Display for OrError<Err1, Err2>
where
    Err1: ErrorNode,
    Err2: ErrorNode<Element = Err1::Element>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.likely_error())
    }
}

impl<Err1, Err2> std::error::Error for OrError<Err1, Err2>
where
    Err1: ErrorNode,
    Err2: ErrorNode<Element = Err1::Element>,
{
}

impl<Err1, Err2> ErrorNode for OrError<Err1, Err2>
where
    Err1: ErrorNode,
    Err2: ErrorNode<Element = Err1::Element>,
{
    type Element = Err1::Element;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        match self {
            OrError::BothFailed { first, second } => {
                let first = first.likely_error();
                let second = second.likely_error();

                if first.loc().position() >= second.loc().position() {
                    first
                } else {
                    second
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

impl<P1, P2> Parser for Or<P1, P2>
where
    P1: Parser,
    P2: Parser<Cursor = P1::Cursor, Output = P1::Output>,
{
    type Cursor = P1::Cursor;
    type Output = P1::Output;
    type Error = OrError<P1::Error, P2::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match self.parser1.parse(cursor) {
            Ok(result) => Ok(result),
            Err(first) => match self.parser2.parse(cursor) {
                Ok(result) => Ok(result),
                Err(second) => Err(OrError::BothFailed { first, second }),
            },
        }
    }
}

/// Extension trait to add .or() method support for parsers
pub trait OrExt: Parser + Sized {
    fn or<P>(self, other: P) -> Or<Self, P>
    where
        P: Parser<Cursor = Self::Cursor, Output = Self::Output>,
    {
        Or::new(self, other)
    }
}

impl<P: Parser> OrExt for P {}

/// Convenience function to create an Or parser
pub fn or<P1, P2>(parser1: P1, parser2: P2) -> Or<P1, P2>
where
    P1: Parser,
    P2: Parser<Cursor = P1::Cursor, Output = P1::Output>,
{
    Or::new(parser1, parser2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::and::AndExt;
    use crate::byte::is_byte;
    use crate::error::{CodeLoc, ParsicombError};
    use crate::filter::FilterExt;
    use crate::map::MapExt;

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
        let furthest = or_error.likely_error();

        assert_eq!(furthest.loc().position(), 2);
        assert!(furthest.to_string().contains("second error"));
    }

    #[test]
    fn test_or_error_furthest_first_wins() {
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
        let furthest = or_error.likely_error();

        assert_eq!(furthest.loc().position(), 3);
        assert!(furthest.to_string().contains("first error"));
    }

    #[test]
    fn test_or_error_auto_recursive_furthest() {
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

        // likely_error() recurses through the nested OrErrors
        let furthest = outer_or.likely_error();

        assert_eq!(furthest.loc().position(), 8);
        assert!(furthest.to_string().contains("error at pos 8"));
    }

    #[test]
    fn test_multiple_nested_ors_furthest_flattening() {
        let data = b"start_test";
        let cursor = ByteCursor::new(data);

        // Create multiple nested or combinators that fail at different positions:
        // This creates a structure like: Or<Or<Or<P1, P2>, P3>, P4>
        // Using map to convert all outputs to u8 so they're compatible
        let parser = is_byte(b'X') // fails at pos 0
            .or(is_byte(b's').and(is_byte(b'Y')).map(|(a, _)| a)) // fails at pos 1
            .or(is_byte(b's')
                .and(is_byte(b't'))
                .and(is_byte(b'Z'))
                .map(|((a, _), _)| a)) // fails at pos 2
            .or(is_byte(b's')
                .and(is_byte(b't'))
                .and(is_byte(b'a'))
                .and(is_byte(b'Q'))
                .map(|(((a, _), _), _)| a)); // fails at pos 3

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // The likely_error() should automatically flatten all the nested Or and And structures
        // and find the error that made it furthest (position 3)
        let error = result.unwrap_err();
        let furthest_error = error.likely_error();

        assert_eq!(
            furthest_error.loc().position(),
            3,
            "likely_error() should traverse nested Or<Or<Or<...>>> and And structures to find the deepest error"
        );
    }

    #[test]
    fn test_comprehensive_error_recursion() {
        let data = b"hello_world";
        let cursor = ByteCursor::new(data);

        // Create a complex nested structure mixing Or, And, Filter, and Map
        // Structure: Or<Filter<And<byte, byte>>, Filter<And<And<byte, byte>, byte>>>
        let branch1 = is_byte(b'h')
            .and(is_byte(b'X')) // fails at pos 1
            .filter(|(_, b)| *b == b'e', "expected 'e' as second byte")
            .map(|(a, _)| a);

        let branch2 = is_byte(b'h')
            .and(is_byte(b'e'))
            .and(is_byte(b'Z')) // fails at pos 2
            .filter(|((_, _), c)| *c == b'l', "expected 'l' as third byte")
            .map(|((a, _), _)| a);

        let parser = branch1.or(branch2);

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // The likely_error() should recursively traverse:
        // OrError -> FilterError -> AndError -> ParsicombError
        // and find the error that got furthest (position 2)
        let error = result.unwrap_err();
        let furthest_error = error.likely_error();

        assert_eq!(
            furthest_error.loc().position(),
            2,
            "likely_error() should traverse complex Or<Filter<And<...>>> structures"
        );
    }
}
