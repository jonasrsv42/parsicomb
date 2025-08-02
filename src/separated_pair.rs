use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::error::{ErrorLeaf, ErrorNode};
use crate::parser::Parser;
use std::fmt;

/// Error type for SeparatedPair parser that can wrap errors from all constituent parsers
#[derive(Debug)]
pub enum SeparatedPairError<E1, ES, E2> {
    /// Error from the left parser
    LeftParser(E1),
    /// Error from the separator parser
    Separator(ES),
    /// Error from the right parser
    RightParser(E2),
}

impl<E1: fmt::Display, ES: fmt::Display, E2: fmt::Display> fmt::Display
    for SeparatedPairError<E1, ES, E2>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeparatedPairError::LeftParser(e) => write!(f, "Left parser failed: {}", e),
            SeparatedPairError::Separator(e) => write!(f, "Separator failed: {}", e),
            SeparatedPairError::RightParser(e) => write!(f, "Right parser failed: {}", e),
        }
    }
}

impl<E1, ES, E2> std::error::Error for SeparatedPairError<E1, ES, E2>
where
    E1: std::error::Error,
    ES: std::error::Error,
    E2: std::error::Error,
{
}

impl<'code, E1, ES, E2, T: Atomic + 'code> ErrorNode<'code> for SeparatedPairError<E1, ES, E2>
where
    E1: ErrorNode<'code, Element = T>,
    ES: ErrorNode<'code, Element = T>,
    E2: ErrorNode<'code, Element = T>,
{
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = T> {
        match self {
            SeparatedPairError::LeftParser(e1) => e1.likely_error(),
            SeparatedPairError::Separator(e) => e.likely_error(),
            SeparatedPairError::RightParser(e2) => e2.likely_error(),
        }
    }
}

/// Parser that matches two values separated by a parser
///
/// This combinator parses: `left + separator + right`
/// and returns a tuple `(left_value, right_value)` with the separator discarded.
///
/// Unlike the UTF-8 whitespace version, this does not handle whitespace automatically.
/// For whitespace handling, use the specific version in `utf8::whitespace::separated_pair`.
///
/// # Examples
/// - `"1.0,2.0"` with separator `,` → `(1.0, 2.0)`
/// - `"hello->world"` with separator `->` → `("hello", "world")`
/// - `"A:B"` with separator `:` → `("A", "B")`
pub struct SeparatedPair<P1, PS, P2> {
    left: P1,
    separator: PS,
    right: P2,
}

impl<P1, PS, P2> SeparatedPair<P1, PS, P2> {
    pub fn new(left: P1, separator: PS, right: P2) -> Self {
        SeparatedPair {
            left,
            separator,
            right,
        }
    }
}

impl<'code, P1, PS, P2> Parser<'code> for SeparatedPair<P1, PS, P2>
where
    P1: Parser<'code>,
    P1::Cursor: Cursor<'code>,
    <P1::Cursor as Cursor<'code>>::Element: Atomic + 'code,
    P1::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element>,
    PS: Parser<'code, Cursor = P1::Cursor>,
    PS::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element>,
    P2: Parser<'code, Cursor = P1::Cursor>,
    P2::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element>,
{
    type Cursor = P1::Cursor;
    type Output = (P1::Output, P2::Output);
    type Error = SeparatedPairError<P1::Error, PS::Error, P2::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        // Parse: left + separator + right
        let (left_val, cursor) = self
            .left
            .parse(cursor)
            .map_err(SeparatedPairError::LeftParser)?;
        let (_, cursor) = self
            .separator
            .parse(cursor)
            .map_err(SeparatedPairError::Separator)?;
        let (right_val, cursor) = self
            .right
            .parse(cursor)
            .map_err(SeparatedPairError::RightParser)?;

        Ok(((left_val, right_val), cursor))
    }
}

/// Creates a parser that matches two values separated by the given parser
///
/// Constraints:
/// - All three parsers must use the same cursor type
/// - All three parsers must have errors with the same element type
pub fn separated_pair<'code, P1, PS, P2>(
    left: P1,
    separator: PS,
    right: P2,
) -> SeparatedPair<P1, PS, P2>
where
    P1: Parser<'code>,
    PS: Parser<'code, Cursor = P1::Cursor>,
    P2: Parser<'code, Cursor = P1::Cursor>,
    P1::Cursor: Cursor<'code>,
    P1::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element>,
    PS::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element>,
    P2::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element>,
    <P1::Cursor as Cursor<'code>>::Element: Atomic + 'code,
{
    SeparatedPair::new(left, separator, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::ascii::number::f64;
    use crate::byte::is_byte;
    use crate::utf8::string::is_string;

    #[test]
    fn test_numbers_no_space() {
        let data = b"1.5,2.7";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_byte(b','), f64());

        let ((left, right), cursor) = parser.parse(cursor).unwrap();
        assert!((left - 1.5).abs() < f64::EPSILON);
        assert!((right - 2.7).abs() < f64::EPSILON);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_strings() {
        let data = b"hello,world";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(is_string("hello"), is_byte(b','), is_string("world"));

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert_eq!(left.as_ref(), "hello");
        assert_eq!(right.as_ref(), "world");
    }

    #[test]
    fn test_mixed_types() {
        let data = b"42.0,test";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_byte(b','), is_string("test"));

        let ((num, text), _) = parser.parse(cursor).unwrap();
        assert!((num - 42.0).abs() < f64::EPSILON);
        assert_eq!(text.as_ref(), "test");
    }

    #[test]
    fn test_no_separator_fails() {
        let data = b"1.0 2.0";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_byte(b','), f64());

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_only_left_value_fails() {
        let data = b"1.0,";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_byte(b','), f64());

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_with_remaining_content() {
        let data = b"1.0,2.0 extra";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_byte(b','), f64());

        let ((left, right), cursor) = parser.parse(cursor).unwrap();
        assert!((left - 1.0).abs() < f64::EPSILON);
        assert!((right - 2.0).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b' ');
    }

    #[test]
    fn test_arrow_separator() {
        let data = b"input->output";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(is_string("input"), is_string("->"), is_string("output"));

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert_eq!(left.as_ref(), "input");
        assert_eq!(right.as_ref(), "output");
    }

    #[test]
    fn test_colon_separator() {
        let data = b"key:value";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(is_string("key"), is_byte(b':'), is_string("value"));

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert_eq!(left.as_ref(), "key");
        assert_eq!(right.as_ref(), "value");
    }

    #[test]
    fn test_different_separator_types() {
        // Using string separator
        let data = b"A::B";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(is_string("A"), is_string("::"), is_string("B"));

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert_eq!(left.as_ref(), "A");
        assert_eq!(right.as_ref(), "B");
    }
}
