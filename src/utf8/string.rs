use crate::ByteCursor;
use crate::Cursor;
use crate::parser::Parser;
use crate::utf8::char::char;
use crate::{CodeLoc, ParsicombError};
use std::borrow::Cow;

// Helper function to reduce error creation boilerplate
fn create_string_error<'code>(
    cursor: &ByteCursor<'code>,
    message: String,
) -> ParsicombError<'code> {
    let (data, position) = cursor.inner();
    ParsicombError::SyntaxError {
        message: message.into(),
        loc: CodeLoc::new(data, position),
    }
}

/// Parser that matches an exact UTF-8 string character by character
pub struct IsStringParser {
    expected: Cow<'static, str>,
}

impl IsStringParser {
    pub fn new(expected: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expected: expected.into(),
        }
    }
}

impl<'code> Parser<'code> for IsStringParser {
    type Cursor = ByteCursor<'code>;
    type Output = Cow<'static, str>;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut current_cursor = cursor;

        for expected_char in self.expected.chars() {
            match char().parse(current_cursor) {
                Ok((parsed_char, new_cursor)) => {
                    if parsed_char == expected_char {
                        current_cursor = new_cursor;
                    } else {
                        return Err(create_string_error(
                            &current_cursor,
                            format!(
                                "expected '{}', found '{}' while matching '{}'",
                                expected_char, parsed_char, self.expected
                            ),
                        ));
                    }
                }
                Err(_) => {
                    return Err(create_string_error(
                        &current_cursor,
                        format!(
                            "expected '{}', but reached end of input while matching '{}'",
                            expected_char, self.expected
                        ),
                    ));
                }
            }
        }

        // Clone is cheap here - just copies the reference for &'static str
        Ok((self.expected.clone(), current_cursor))
    }
}

/// Convenience function to create an IsStringParser
pub fn is_string(expected: impl Into<Cow<'static, str>>) -> IsStringParser {
    IsStringParser::new(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

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
                .contains("expected 'h', found 'w'")
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
                .contains("expected 'l', found 'p'")
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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("reached end of input")
        );
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
                .contains("reached end of input")
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
                .contains("expected 'h', found 'H'")
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
