use crate::ByteCursor;
use crate::ParsicombError;
use crate::atomic::{IsSequence, is_sequence};
use crate::parser::Parser;
use std::borrow::Cow;

/// Parser that matches an exact UTF-8 string
///
/// Failures don't allocate for `&'static str`; an owned string is cloned
/// into each error.
pub struct IsStringParser<'code> {
    expected: Cow<'static, str>,
    bytes: IsSequence<ByteCursor<'code>>,
}

impl<'code> IsStringParser<'code> {
    pub fn new(expected: impl Into<Cow<'static, str>>) -> Self {
        let expected = expected.into();
        let bytes: Cow<'static, [u8]> = match &expected {
            Cow::Borrowed(s) => Cow::Borrowed(s.as_bytes()),
            Cow::Owned(s) => Cow::Owned(s.as_bytes().to_vec()),
        };

        Self {
            expected,
            bytes: is_sequence(bytes),
        }
    }
}

impl<'code> Parser for IsStringParser<'code> {
    type Cursor = ByteCursor<'code>;
    type Output = Cow<'static, str>;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (_, next_cursor) = self.bytes.parse(cursor)?;
        // Cheap for `&'static str` (the common case): clones the reference
        Ok((self.expected.clone(), next_cursor))
    }
}

/// Convenience function to create an IsStringParser
pub fn is_string<'code>(expected: impl Into<Cow<'static, str>>) -> IsStringParser<'code> {
    IsStringParser::new(expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utf8::char::char;

    #[test]
    fn test_exact_match() {
        let input = "hello";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello");

        let (result, _) = parser.parse(cursor).unwrap();
        assert_eq!(result.as_ref(), "hello");
    }

    #[test]
    fn test_partial_match_with_remaining() {
        let input = "hello world";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello");

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result.as_ref(), "hello");

        // Should stop after "hello", next char should be space
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, ' ');
    }

    #[test]
    fn test_unicode_string() {
        let input = "こんにちは世界";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("こんにちは");

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result.as_ref(), "こんにちは");

        // Should have remaining content "世界"
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, '世');
    }

    #[test]
    fn test_empty_string() {
        let input = "hello";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("");

        let (result, cursor_after) = parser.parse(cursor).unwrap();
        assert_eq!(result.as_ref(), "");
        // Cursor should not advance for empty string
        let (_data1, pos1) = cursor.inner();
        let (_data2, pos2) = cursor_after.inner();
        assert_eq!(pos1, pos2);
    }

    #[test]
    fn test_mismatch_first_char() {
        let input = "world";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello");

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains(r#"expected "hello", found "w""#)
        );
    }

    #[test]
    fn test_mismatch_middle_char() {
        let input = "help";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello");

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains(r#"expected "hello", found "help""#)
        );
    }

    #[test]
    fn test_insufficient_input() {
        let input = "hel";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello");

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at end of input"));
    }

    #[test]
    fn test_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello");

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("found end of input")
        );
    }

    #[test]
    fn test_case_sensitive() {
        let input = "Hello";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello");

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains(r#"expected "hello", found "H""#)
        );
    }

    #[test]
    fn test_programming_keywords() {
        let test_cases = [
            "true", "false", "struct", "type", "fn", "if", "else", "while", "for", "let", "const",
        ];

        for keyword in test_cases {
            let data = keyword.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = is_string(keyword);

            let (result, _) = parser.parse(cursor).unwrap();
            assert_eq!(result.as_ref(), keyword, "Failed for keyword: {}", keyword);
        }
    }

    #[test]
    fn test_mixed_unicode_content() {
        let test_cases = ["température", "变量", "αβγ", "🚀🦀"];

        for expected in test_cases {
            let data = expected.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = is_string(expected);

            let (result, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                result.as_ref(),
                expected,
                "Failed for Unicode string: {}",
                expected
            );
        }
    }

    #[test]
    fn test_operators_and_symbols() {
        let test_cases = [
            "<-", "->", "==", "!=", "<=", ">=", "::", "&&", "||", "++", "--",
        ];

        for symbol in test_cases {
            let data = symbol.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = is_string(symbol);

            let (result, _) = parser.parse(cursor).unwrap();
            assert_eq!(result.as_ref(), symbol, "Failed for symbol: {}", symbol);
        }
    }

    #[test]
    fn test_emoji_sequences() {
        // Test complex emoji sequences
        let input = "👨‍💻🔥";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("👨‍💻");

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result.as_ref(), "👨‍💻");

        // Should have remaining emoji
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, '🔥');
    }
}
