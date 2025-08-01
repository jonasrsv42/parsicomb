use super::parser::Parser;
use crate::atomic::Atomic;
use crate::error::{ErrorLeaf, ErrorNode};
use std::fmt;

// # Or Combinator - Dynamic Dispatch for Compile Time Performance
//
// ## Why We Use `Box<dyn Parser>` and `Box<dyn ErrorNode>`
//
// This combinator uses dynamic dispatch (trait objects) to solve a critical compile-time
// performance issue. Without boxing, chaining `.or()` calls creates exponentially complex types:
//
// ```ignore
// // Without boxing, this creates nested types:
// // Or<Or<Or<P1, P2>, P3>, P4>
// // With error types: OrError<OrError<OrError<E1, E2>, E3>, E4>
//
// let parser = a.or(b).or(c).or(d).or(e); // Gets progressively worse
// ```
//
// **The Problem**: Deep generic nesting causes:
// - Exponential compile times (we've seen infinite compilation in downstream crates)
// - Exponential memory usage during compilation
// - Unreadable error messages
// - Inability to express recursive grammars
//
// **The Solution**: Dynamic dispatch flattens all chains to:
// ```ignore
// // With boxing: Always just Or<'code, Cursor, Output, E1, E2>
// // With error: Always just OrError<'code>
// ```
//
// ## Performance Trade-offs
//
// **Cost**: One additional heap allocation per combinator + virtual dispatch
// **Benefit**: Eliminates compile-time explosion, enables recursive parsers
// **Result**: Faster development iteration, ability to parse complex grammars
//
// ## Error Handling Strategy
//
// Errors are stored as `Box<dyn ErrorNode>` and only converted to concrete types
// when displaying errors (via `likely_error()`). This avoids cloning during
// error propagation while preserving full error information.

/// Error type for Or parser that can wrap errors from both parsers when both fail
pub enum OrError<'code, T: Atomic> {
    /// Both parsers failed
    BothFailed {
        first: Box<dyn ErrorNode<'code, Element = T> + 'code>,
        second: Box<dyn ErrorNode<'code, Element = T> + 'code>,
    },
}

impl<'code, T: Atomic> std::fmt::Debug for OrError<'code, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrError::BothFailed { first, second } => f
                .debug_struct("BothFailed")
                .field("first", &format!("{}", &**first))
                .field("second", &format!("{}", &**second))
                .finish(),
        }
    }
}

impl<'code, T: Atomic> fmt::Display for OrError<'code, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrError::BothFailed { first, second } => {
                write!(
                    f,
                    "Both parsers failed - First: {}, Second: {}",
                    &**first, &**second
                )
            }
        }
    }
}

impl<'code, T: Atomic> std::error::Error for OrError<'code, T> {}

// OrError implements ErrorNode to enable furthest-error selection
impl<'code, T: Atomic + 'code> ErrorNode<'code> for OrError<'code, T> {
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = Self::Element> {
        match self {
            OrError::BothFailed { first, second } => {
                let first_base = first.as_ref().likely_error();
                let second_base = second.as_ref().likely_error();

                if first_base.loc().position() >= second_base.loc().position() {
                    first_base
                } else {
                    second_base
                }
            }
        }
    }
}

/// Parser combinator that tries the first parser, and if it fails, tries the second parser
pub struct Or<'code, C, O, E1, E2> {
    parser1: Box<dyn Parser<'code, Cursor = C, Output = O, Error = E1> + 'code>,
    parser2: Box<dyn Parser<'code, Cursor = C, Output = O, Error = E2> + 'code>,
}

impl<'code, C, O, E1, E2> Or<'code, C, O, E1, E2> {
    pub fn new<P1, P2>(parser1: P1, parser2: P2) -> Self
    where
        P1: Parser<'code, Cursor = C, Output = O, Error = E1> + 'code,
        P2: Parser<'code, Cursor = C, Output = O, Error = E2> + 'code,
    {
        Or {
            parser1: Box::new(parser1),
            parser2: Box::new(parser2),
        }
    }
}

impl<'code, C, O, E1, E2> Parser<'code> for Or<'code, C, O, E1, E2>
where
    C: crate::cursors::Cursor<'code>,
    C::Element: Atomic + 'code,
    E1: std::error::Error + ErrorNode<'code, Element = C::Element> + 'code,
    E2: std::error::Error + ErrorNode<'code, Element = C::Element> + 'code,
{
    type Cursor = C;
    type Output = O;
    type Error = OrError<'code, C::Element>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match self.parser1.parse(cursor) {
            Ok(result) => Ok(result),
            Err(first_error) => match self.parser2.parse(cursor) {
                Ok(result) => Ok(result),
                Err(second_error) => Err(OrError::BothFailed {
                    first: Box::new(first_error),
                    second: Box::new(second_error),
                }),
            },
        }
    }
}

/// Extension trait to add .or() method support for parsers
pub trait OrExt<'code>: Parser<'code> + Sized {
    fn or<P>(self, other: P) -> Or<'code, Self::Cursor, Self::Output, Self::Error, P::Error>
    where
        P: Parser<'code, Output = Self::Output, Cursor = Self::Cursor> + 'code,
        Self: 'code,
    {
        Or::new(self, other)
    }
}

/// Implement OrExt for all parsers
impl<'code, P> OrExt<'code> for P where P: Parser<'code> {}

/// Convenience function to create an Or parser
pub fn or<'code, P1, P2>(
    parser1: P1,
    parser2: P2,
) -> Or<'code, P1::Cursor, P1::Output, P1::Error, P2::Error>
where
    P1: Parser<'code> + 'code,
    P2: Parser<'code, Output = P1::Output, Cursor = P1::Cursor> + 'code,
{
    Or::new(parser1, parser2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cursor;
    use crate::and::AndExt;
    use crate::byte::is_byte;
    use crate::byte_cursor::ByteCursor;
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
            first: Box::new(error1),
            second: Box::new(error2),
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
            first: Box::new(error1),
            second: Box::new(error2),
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
            first: Box::new(error1),
            second: Box::new(error2),
        };
        let outer_or = OrError::BothFailed {
            first: Box::new(inner_or),
            second: Box::new(error3),
        };

        // Use the new ErrorBranch system - this automatically handles recursion!
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

        // The furthest() should automatically flatten all the nested Or and And structures
        // and find the error that made it furthest (position 3)
        let error = result.unwrap_err();
        let furthest_error = error.likely_error();

        assert_eq!(
            furthest_error.loc().position(),
            3,
            "furthest() should traverse nested Or<Or<Or<...>>> and And structures to find the deepest error"
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

        // The furthest() should recursively traverse:
        // OrError -> FilterError -> AndError -> ParsicombError
        // and find the error that got furthest (position 2)
        let error = result.unwrap_err();
        let furthest_error = error.likely_error();

        assert_eq!(
            furthest_error.loc().position(),
            2,
            "furthest() should traverse complex Or<Filter<And<...>>> structures"
        );
    }
}
