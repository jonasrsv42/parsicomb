use crate::ByteCursor;
use crate::Cursor;
use crate::ParsicombError;
use crate::error::Expected;
use crate::parser::Parser;
use crate::utf8::char::char;
use std::borrow::Cow;
use std::marker::PhantomData;

/// Parser that matches an exact UTF-8 string
pub struct IsStringParser<'code> {
    expected: Cow<'static, str>,
    _input: PhantomData<&'code [u8]>,
}

impl<'code> IsStringParser<'code> {
    pub fn new(expected: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expected: expected.into(),
            _input: PhantomData,
        }
    }

    /// Build the mismatch error at `cursor`, decoding the character found there
    fn mismatch(&self, cursor: ByteCursor<'code>, expected: char) -> ParsicombError<'code> {
        ParsicombError::Expected {
            expected: Expected::CharInString {
                expected,
                found: char().parse(cursor).ok().map(|(found, _)| found),
                string: self.expected.clone(),
            },
            loc: cursor.loc(),
        }
    }
}

impl<'code> Parser for IsStringParser<'code> {
    type Cursor = ByteCursor<'code>;
    type Output = Cow<'static, str>;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (source, start) = cursor.inner();
        let expected = self.expected.as_bytes();

        if source[start..].starts_with(expected) {
            // Advance past the match; `expected` is valid UTF-8, so this lands
            // on a character boundary
            let mut end = cursor;
            for _ in 0..expected.len() {
                end = end.next();
            }
            return Ok((self.expected.clone(), end));
        }

        // Report the first character that differs
        let mut current = cursor;
        for expected_char in self.expected.chars() {
            match char().parse(current) {
                Ok((found, next)) if found == expected_char => current = next,
                _ => return Err(self.mismatch(current, expected_char)),
            }
        }

        // Unreachable when the prefix check failed, but fail rather than panic
        Err(self.mismatch(current, self.expected.chars().next().unwrap_or_default()))
    }
}

/// Convenience function to create an IsStringParser
pub fn is_string<'code>(expected: impl Into<Cow<'static, str>>) -> IsStringParser<'code> {
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
