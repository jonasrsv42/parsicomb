use super::parser::Parser;
use crate::atomic::Atomic;
use crate::error::{ErrorLeaf, ErrorNode};
use std::fmt;

// # And Combinator - Dynamic Dispatch for Compile Time Performance
//
// ## Why We Use `Box<dyn Parser>` and `Box<dyn ErrorNode>`
//
// Like the Or combinator, this uses dynamic dispatch to prevent compile-time explosion.
// Without boxing, chaining `.and()` calls creates exponentially complex nested types:
//
// ```ignore
// // Without boxing, this creates deeply nested types:
// // And<And<And<P1, P2>, P3>, P4>
// // With nested tuples: (((O1, O2), O3), O4)
// // With nested errors: AndError<AndError<AndError<E1, E2>, E3>, E4>
//
// let parser = a.and(b).and(c).and(d).and(e); // Exponential complexity
// ```
//
// **The Problem**: The same issues as Or combinator:
// - Exponential compile times leading to infinite compilation
// - Exponential memory usage during compilation
// - Type system limitations with deep recursion
// - Poor error messages from the compiler
//
// **The Solution**: Dynamic dispatch flattens to manageable types:
// ```ignore
// // With boxing: Always just And<'code, Cursor, Output1, Output2, E1, E2>
// // With error: Always just AndError<'code>
// ```
//
// ## Performance Characteristics
//
// **Runtime Cost**: Small heap allocation + virtual dispatch per combinator
// **Compile Cost**: Dramatically reduced - enables complex parser chains
// **Memory**: Constant per combinator instead of exponential growth
//
// ## Error Strategy
//
// Uses the same efficient error handling as Or: errors are boxed and only
// resolved via `likely_error()` when actually needed for display, avoiding
// unnecessary cloning during error propagation.

/// Error type for And parser that can wrap errors from either the first or second parser
pub enum AndError<'code, T: Atomic> {
    /// Error from the first parser
    FirstParser(Box<dyn ErrorNode<'code, Element = T> + 'code>),
    /// Error from the second parser
    SecondParser(Box<dyn ErrorNode<'code, Element = T> + 'code>),
}

impl<'code, T: Atomic> std::fmt::Debug for AndError<'code, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AndError::FirstParser(e) => f
                .debug_tuple("FirstParser")
                .field(&format!("{}", &**e))
                .finish(),
            AndError::SecondParser(e) => f
                .debug_tuple("SecondParser")
                .field(&format!("{}", &**e))
                .finish(),
        }
    }
}

// Note: into_inner method removed since we now use boxed trait objects

impl<'code, T: Atomic> fmt::Display for AndError<'code, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AndError::FirstParser(e) => write!(f, "First parser failed: {}", &**e),
            AndError::SecondParser(e) => write!(f, "Second parser failed: {}", &**e),
        }
    }
}

impl<'code, T: Atomic> std::error::Error for AndError<'code, T> {}

// Note: From implementation removed to avoid calling likely_error() internally

// Implement ErrorNode for AndError to enable furthest-error selection in nested structures
impl<'code, T: Atomic + 'code> ErrorNode<'code> for AndError<'code, T> {
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = Self::Element> {
        match self {
            // First parser failed - return its error
            AndError::FirstParser(e1) => e1.as_ref().likely_error(),
            // Second parser failed - this means first parser succeeded and advanced the cursor,
            // so the second parser's error is further in the input
            AndError::SecondParser(e2) => e2.as_ref().likely_error(),
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
pub struct And<'code, C, O1, O2, E1, E2> {
    parser1: Box<dyn Parser<'code, Cursor = C, Output = O1, Error = E1> + 'code>,
    parser2: Box<dyn Parser<'code, Cursor = C, Output = O2, Error = E2> + 'code>,
}

impl<'code, C, O1, O2, E1, E2> And<'code, C, O1, O2, E1, E2> {
    pub fn new<P1, P2>(parser1: P1, parser2: P2) -> Self
    where
        P1: Parser<'code, Cursor = C, Output = O1, Error = E1> + 'code,
        P2: Parser<'code, Cursor = C, Output = O2, Error = E2> + 'code,
    {
        And {
            parser1: Box::new(parser1),
            parser2: Box::new(parser2),
        }
    }
}

impl<'code, C, O1, O2, E1, E2> Parser<'code> for And<'code, C, O1, O2, E1, E2>
where
    C: crate::cursors::Cursor<'code>,
    C::Element: Atomic + 'code,
    E1: std::error::Error + ErrorNode<'code, Element = C::Element> + 'code,
    E2: std::error::Error + ErrorNode<'code, Element = C::Element> + 'code,
{
    type Cursor = C;
    type Output = (O1, O2);
    type Error = AndError<'code, C::Element>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (result1, cursor) = self
            .parser1
            .parse(cursor)
            .map_err(|e| AndError::FirstParser(Box::new(e)))?;
        let (result2, cursor) = self
            .parser2
            .parse(cursor)
            .map_err(|e| AndError::SecondParser(Box::new(e)))?;
        Ok(((result1, result2), cursor))
    }
}

/// Convenience function to create an And parser
pub fn and<'code, P1, P2>(
    parser1: P1,
    parser2: P2,
) -> And<'code, P1::Cursor, P1::Output, P2::Output, P1::Error, P2::Error>
where
    P1: Parser<'code> + 'code,
    P2: Parser<'code, Cursor = P1::Cursor> + 'code,
{
    And::new(parser1, parser2)
}

/// Extension trait to add .and() method support for parsers
pub trait AndExt<'code>: Parser<'code> + Sized {
    fn and<P>(
        self,
        other: P,
    ) -> And<'code, Self::Cursor, Self::Output, P::Output, Self::Error, P::Error>
    where
        P: Parser<'code, Cursor = Self::Cursor> + 'code,
        Self: 'code,
    {
        And::new(self, other)
    }
}

/// Implement AndExt for all parsers
impl<'code, P> AndExt<'code> for P where P: Parser<'code> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cursor;
    use crate::ascii::i64;
    use crate::byte::is_byte;
    use crate::byte_cursor::ByteCursor;

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
