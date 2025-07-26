use super::unicode_whitespace;
use crate::ParsicombError;
use crate::byte_cursor::ByteCursor;
use crate::error::{ErrorLeaf, ErrorNode};
use crate::filter::FilterError;
use crate::many::many;
use crate::parser::Parser;
use std::fmt;

/// Error type for SeparatedPair parser that can wrap errors from all constituent parsers
#[derive(Debug)]
pub enum SeparatedPairError<'code, E1, ES, E2> {
    /// Error from the left parser
    LeftParser(E1),
    /// Error from whitespace after left parser
    LeftWhitespace(FilterError<'code, ParsicombError<'code>>),
    /// Error from the separator parser
    Separator(ES),
    /// Error from whitespace after separator
    RightWhitespace(FilterError<'code, ParsicombError<'code>>),
    /// Error from the right parser
    RightParser(E2),
}

impl<E1: fmt::Display, ES: fmt::Display, E2: fmt::Display> fmt::Display
    for SeparatedPairError<'_, E1, ES, E2>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeparatedPairError::LeftParser(e) => write!(f, "Left parser failed: {}", e),
            SeparatedPairError::LeftWhitespace(e) => write!(f, "Left whitespace failed: {}", e),
            SeparatedPairError::Separator(e) => write!(f, "Separator failed: {}", e),
            SeparatedPairError::RightWhitespace(e) => write!(f, "Right whitespace failed: {}", e),
            SeparatedPairError::RightParser(e) => write!(f, "Right parser failed: {}", e),
        }
    }
}

impl<E1, ES, E2> std::error::Error for SeparatedPairError<'_, E1, ES, E2>
where
    E1: std::error::Error,
    ES: std::error::Error,
    E2: std::error::Error,
{
}

// Implement ErrorBranch for SeparatedPairError to enable furthest-error selection
impl<'code, E1, ES, E2> ErrorNode<'code> for SeparatedPairError<'code, E1, ES, E2>
where
    E1: ErrorNode<'code>,
    ES: ErrorNode<'code>,
    E2: ErrorNode<'code>,
{
    fn likely_error(self) -> Box<dyn ErrorLeaf + 'code> {
        match self {
            SeparatedPairError::LeftParser(e1) => e1.likely_error(),
            SeparatedPairError::LeftWhitespace(e) => e.likely_error(),
            SeparatedPairError::Separator(e) => e.likely_error(),
            SeparatedPairError::RightWhitespace(e) => e.likely_error(),
            SeparatedPairError::RightParser(e2) => e2.likely_error(),
        }
    }
}

/// Parser that matches two values separated by a parser with optional whitespace
///
/// This combinator automatically handles Unicode whitespace around the separator.
/// It parses: `left + optional_ws + separator + optional_ws + right`
///
/// # Returns
/// A tuple `(left_value, right_value)` with the separator and whitespace discarded.
///
/// # Examples
/// - `"1.0,2.0"` with separator `is_byte(b',')` → `(1.0, 2.0)`
/// - `"1.0 , 2.0"` with separator `is_string(",")` → `(1.0, 2.0)`
/// - `"hello -> world"` with separator `is_string("->")` → `("hello", "world")`
/// Custom SeparatedPair parser implementation
pub struct SeparatedPair<P1, PS, P2> {
    left: P1,
    separator: PS,
    right: P2,
}

impl<'code, P1, PS, P2> Parser<'code> for SeparatedPair<P1, PS, P2>
where
    P1: Parser<'code>,
    PS: Parser<'code>,
    P2: Parser<'code>,
{
    type Output = (P1::Output, P2::Output);
    type Error = SeparatedPairError<'code, P1::Error, PS::Error, P2::Error>;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
        // Parse: left + whitespace + separator + whitespace + right
        let (left_val, cursor) = self
            .left
            .parse(cursor)
            .map_err(SeparatedPairError::LeftParser)?;
        let (_, cursor) = many(unicode_whitespace())
            .parse(cursor)
            .map_err(|e| SeparatedPairError::LeftWhitespace(e))?;
        let (_, cursor) = self
            .separator
            .parse(cursor)
            .map_err(SeparatedPairError::Separator)?;
        let (_, cursor) = many(unicode_whitespace())
            .parse(cursor)
            .map_err(|e| SeparatedPairError::RightWhitespace(e))?;
        let (right_val, cursor) = self
            .right
            .parse(cursor)
            .map_err(SeparatedPairError::RightParser)?;

        Ok(((left_val, right_val), cursor))
    }
}

pub fn separated_pair<'code, P1, PS, P2>(
    left: P1,
    separator: PS,
    right: P2,
) -> SeparatedPair<P1, PS, P2>
where
    P1: Parser<'code>,
    PS: Parser<'code>,
    P2: Parser<'code>,
{
    SeparatedPair {
        left,
        separator,
        right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ascii::number::f64;
    use crate::byte_cursor::ByteCursor;
    use crate::utf8::string::is_string;

    #[test]
    fn test_numbers_no_space() {
        let data = b"1.5,2.7";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_string(","), f64());

        let ((left, right), cursor) = parser.parse(cursor).unwrap();
        assert!((left - 1.5).abs() < f64::EPSILON);
        assert!((right - 2.7).abs() < f64::EPSILON);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_numbers_with_spaces() {
        let data = b"3.14  ,  2.71";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_string(","), f64());

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert!((left - 3.14).abs() < f64::EPSILON);
        assert!((right - 2.71).abs() < f64::EPSILON);
    }

    #[test]
    fn test_strings() {
        let data = b"hello , world";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(is_string("hello"), is_string(","), is_string("world"));

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert_eq!(left.as_ref(), "hello");
        assert_eq!(right.as_ref(), "world");
    }

    #[test]
    fn test_mixed_types() {
        let data = b"42.0 , test";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_string(","), is_string("test"));

        let ((num, text), _) = parser.parse(cursor).unwrap();
        assert!((num - 42.0).abs() < f64::EPSILON);
        assert_eq!(text.as_ref(), "test");
    }

    #[test]
    fn test_unicode_whitespace() {
        // Use various Unicode whitespace characters
        let input = "1.0\u{2000},\u{3000}2.0"; // En quad + Ideographic space
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_string(","), f64());

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert!((left - 1.0).abs() < f64::EPSILON);
        assert!((right - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_comma_fails() {
        let data = b"1.0 2.0";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_string(","), f64());

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_only_left_value_fails() {
        let data = b"1.0,";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_string(","), f64());

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_with_remaining_content() {
        let data = b"1.0, 2.0 extra";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_string(","), f64());

        let ((left, right), cursor) = parser.parse(cursor).unwrap();
        assert!((left - 1.0).abs() < f64::EPSILON);
        assert!((right - 2.0).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b' ');
    }

    #[test]
    fn test_arrow_separator() {
        let data = b"input -> output";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(is_string("input"), is_string("->"), is_string("output"));

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert_eq!(left.as_ref(), "input");
        assert_eq!(right.as_ref(), "output");
    }

    #[test]
    fn test_byte_separator() {
        use crate::byte::is_byte;

        let data = b"1.5,2.7";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), is_byte(b','), f64());

        let ((left, right), cursor) = parser.parse(cursor).unwrap();
        assert!((left - 1.5).abs() < f64::EPSILON);
        assert!((right - 2.7).abs() < f64::EPSILON);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
