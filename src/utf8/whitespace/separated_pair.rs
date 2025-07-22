use super::unicode_whitespace;
use crate::and::AndExt;
use crate::many::many;
use crate::map::MapExt;
use crate::parser::Parser;
use crate::utf8::string::is_string;

/// Parser that matches two values separated by a string separator with optional whitespace
///
/// This combinator automatically handles Unicode whitespace around the separator.
/// It parses: `left + optional_ws + separator + optional_ws + right`
///
/// # Returns
/// A tuple `(left_value, right_value)` with the separator and whitespace discarded.
///
/// # Examples
/// - `"1.0,2.0"` with separator `","` → `(1.0, 2.0)`
/// - `"1.0 , 2.0"` with separator `","` → `(1.0, 2.0)`
/// - `"hello -> world"` with separator `"->"` → `("hello", "world")`
pub fn separated_pair<'a, P1, P2, S>(
    left: P1,
    separator: S,
    right: P2,
) -> impl Parser<'a, Output = (P1::Output, P2::Output)>
where
    P1: Parser<'a>,
    P2: Parser<'a>,
    S: Into<std::borrow::Cow<'static, str>>,
{
    let separator = separator.into();
    left.and(many(unicode_whitespace()))
        .and(is_string(separator))
        .and(many(unicode_whitespace()))
        .and(right)
        .map(|((((left_val, _), _), _), right_val)| (left_val, right_val))
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
        let parser = separated_pair(f64(), ",", f64());

        let ((left, right), cursor) = parser.parse(cursor).unwrap();
        assert!((left - 1.5).abs() < f64::EPSILON);
        assert!((right - 2.7).abs() < f64::EPSILON);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_numbers_with_spaces() {
        let data = b"3.14  ,  2.71";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), ",", f64());

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert!((left - 3.14).abs() < f64::EPSILON);
        assert!((right - 2.71).abs() < f64::EPSILON);
    }

    #[test]
    fn test_strings() {
        let data = b"hello , world";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(is_string("hello"), ",", is_string("world"));

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert_eq!(left.as_ref(), "hello");
        assert_eq!(right.as_ref(), "world");
    }

    #[test]
    fn test_mixed_types() {
        let data = b"42.0 , test";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), ",", is_string("test"));

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
        let parser = separated_pair(f64(), ",", f64());

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert!((left - 1.0).abs() < f64::EPSILON);
        assert!((right - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_comma_fails() {
        let data = b"1.0 2.0";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), ",", f64());

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_only_left_value_fails() {
        let data = b"1.0,";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), ",", f64());

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_with_remaining_content() {
        let data = b"1.0, 2.0 extra";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(f64(), ",", f64());

        let ((left, right), cursor) = parser.parse(cursor).unwrap();
        assert!((left - 1.0).abs() < f64::EPSILON);
        assert!((right - 2.0).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b' ');
    }

    #[test]
    fn test_arrow_separator() {
        let data = b"input -> output";
        let cursor = ByteCursor::new(data);
        let parser = separated_pair(is_string("input"), "->", is_string("output"));

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert_eq!(left.as_ref(), "input");
        assert_eq!(right.as_ref(), "output");
    }
}
