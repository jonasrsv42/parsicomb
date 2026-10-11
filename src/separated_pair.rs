//! Two parsers separated by a third

use crate::parser::Parser;

use crate::error::composite_error;

composite_error! {
    /// Error type for SeparatedPair parser that can wrap errors from all constituent parsers
    pub enum SeparatedPairError<Err1, ErrSep, Err2> {
        /// Error from the left parser
        LeftParser(Err1),
        /// Error from the separator parser
        Separator(ErrSep),
        /// Error from the right parser
        RightParser(Err2),
    }
}

/// Parser that matches two values separated by a parser
///
/// This combinator parses: `left + separator + right`
/// and returns a tuple `(left_value, right_value)` with the separator discarded.
///
/// It does not handle whitespace.
///
/// # Examples
/// - `"1.0,2.0"` with separator `,` → `(1.0, 2.0)`
/// - `"hello->world"` with separator `->` → `("hello", "world")`
/// - `"A:B"` with separator `:` → `("A", "B")`
pub struct SeparatedPair<P1, PSep, P2> {
    left: P1,
    separator: PSep,
    right: P2,
}

impl<P1, PSep, P2> SeparatedPair<P1, PSep, P2> {
    /// Same as [`separated_pair`]
    pub fn new(left: P1, separator: PSep, right: P2) -> Self {
        SeparatedPair {
            left,
            separator,
            right,
        }
    }
}

impl<P1, PSep, P2> Parser for SeparatedPair<P1, PSep, P2>
where
    P1: Parser,
    PSep: Parser<Cursor = P1::Cursor>,
    P2: Parser<Cursor = P1::Cursor>,
{
    type Cursor = P1::Cursor;
    type Output = (P1::Output, P2::Output);
    type Error = SeparatedPairError<P1::Error, PSep::Error, P2::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
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
pub fn separated_pair<P1, PSep, P2>(
    left: P1,
    separator: PSep,
    right: P2,
) -> SeparatedPair<P1, PSep, P2>
where
    P1: Parser,
    PSep: Parser<Cursor = P1::Cursor>,
    P2: Parser<Cursor = P1::Cursor>,
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
