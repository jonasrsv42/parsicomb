use super::unicode_whitespace;
use crate::and::AndExt;
use crate::many::many;
use crate::map::MapExt;
use crate::parser::Parser;

/// Parser that matches content between opening and closing delimiters with automatic whitespace handling
///
/// This combinator automatically handles Unicode whitespace around the content.
/// It parses: `open + optional_ws + content + optional_ws + close`
///
/// # Returns
/// Just the `content` value with the delimiters and whitespace discarded.
///
/// # Examples
/// - `"[1.0]"` → `1.0`
/// - `"[ 1.0 ]"` → `1.0`  
/// - `"(hello)"` → `"hello"`
/// - `"{ content }"` → `"content"`
pub fn between<'a, P1, P2, P3>(
    open: P1,
    content: P2,
    close: P3,
) -> impl Parser<'a, Output = P2::Output>
where
    P1: Parser<'a>,
    P2: Parser<'a>,
    P3: Parser<'a>,
{
    open.and(many(unicode_whitespace()))
        .and(content)
        .and(many(unicode_whitespace()))
        .and(close)
        .map(|((((_, _), content_val), _), _)| content_val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ascii::number::f64;
    use crate::byte::is_byte;
    use crate::byte_cursor::ByteCursor;
    use crate::utf8::string::is_string;
    use crate::utf8::whitespace::separated_pair;

    #[test]
    fn test_brackets_number() {
        let data = b"[42.5]";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - 42.5).abs() < f64::EPSILON);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_brackets_with_spaces() {
        let data = b"[  3.14  ]";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert!((value - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parentheses_string() {
        let data = b"( hello )";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(is_byte(b'('), is_string("hello"), is_byte(b')'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert_eq!(value.as_ref(), "hello");
    }

    #[test]
    fn test_braces() {
        let data = b"{test}";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(is_byte(b'{'), is_string("test"), is_byte(b'}'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert_eq!(value.as_ref(), "test");
    }

    #[test]
    fn test_nested_with_separated_pair() {
        // Test the combination we'll use for intervals: [1.0, 2.0]
        let data = b"[1.0, 2.0]";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(
            is_byte(b'['),
            separated_pair(f64(), ",", f64()),
            is_byte(b']'),
        );

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert!((left - 1.0).abs() < f64::EPSILON);
        assert!((right - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_nested_with_extra_whitespace() {
        let data = b"[  1.5  ,  2.5  ]";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(
            is_byte(b'['),
            separated_pair(f64(), ",", f64()),
            is_byte(b']'),
        );

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert!((left - 1.5).abs() < f64::EPSILON);
        assert!((right - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_unicode_whitespace() {
        // Use various Unicode whitespace characters
        let input = "[\u{2000}42.0\u{3000}]"; // En quad + Ideographic space
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert!((value - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_missing_open_delimiter_fails() {
        let data = b"42.0]";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_missing_close_delimiter_fails() {
        let data = b"[42.0";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_with_remaining_content() {
        let data = b"[42.0] extra";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - 42.0).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b' ');
    }
}

