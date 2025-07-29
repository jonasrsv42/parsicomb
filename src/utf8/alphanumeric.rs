use crate::byte_cursor::ByteCursor;
use crate::filter::FilterExt;
use crate::parser::Parser;
use crate::utf8::char::char;

/// Convenience function to create a Unicode alphanumeric parser
pub fn unicode_alphanumeric()
-> impl for<'code> Parser<'code, Cursor = ByteCursor<'code>, Output = char> {
    char().filter(|c| c.is_alphanumeric(), "expected Unicode letter or digit")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_cursor::ByteCursor;

    #[test]
    fn test_ascii_alphanumeric() {
        // Test ASCII letters
        for ch in 'a'..='z' {
            let input = ch.to_string();
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_alphanumeric();

            let (result_ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(result_ch, ch, "Failed for ASCII lowercase: {}", ch);
        }

        for ch in 'A'..='Z' {
            let input = ch.to_string();
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_alphanumeric();

            let (result_ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(result_ch, ch, "Failed for ASCII uppercase: {}", ch);
        }

        // Test ASCII digits
        for ch in '0'..='9' {
            let input = ch.to_string();
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_alphanumeric();

            let (result_ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(result_ch, ch, "Failed for ASCII digit: {}", ch);
        }
    }

    #[test]
    fn test_unicode_letters() {
        let test_cases = [
            // Latin extended
            ("ñ", 'ñ'),
            ("ü", 'ü'),
            ("ß", 'ß'),
            ("ç", 'ç'),
            // Greek
            ("α", 'α'),
            ("Ω", 'Ω'),
            ("π", 'π'),
            ("Σ", 'Σ'),
            // Cyrillic
            ("а", 'а'),
            ("Я", 'Я'),
            ("ж", 'ж'),
            ("Щ", 'Щ'),
            // CJK
            ("中", '中'),
            ("文", '文'),
            ("あ", 'あ'),
            ("ア", 'ア'),
            ("가", '가'),
        ];

        for (input, expected) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_alphanumeric();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected,
                "Failed for Unicode letter: {} (U+{:04X})",
                input, expected as u32
            );
        }
    }

    #[test]
    fn test_unicode_digits() {
        let test_cases = [
            // Arabic-Indic digits
            ("٠", '٠'),
            ("٥", '٥'),
            ("٩", '٩'),
            // Devanagari digits
            ("०", '०'),
            ("५", '५'),
            ("९", '९'),
            // Fullwidth digits
            ("０", '０'),
            ("５", '５'),
            ("９", '９'),
        ];

        for (input, expected) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_alphanumeric();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected,
                "Failed for Unicode digit: {} (U+{:04X})",
                input, expected as u32
            );
        }
    }

    #[test]
    fn test_mixed_alphanumeric_sequence() {
        let input = "a5中９ñ٠";
        let data = input.as_bytes();
        let mut cursor = ByteCursor::new(data);
        let parser = unicode_alphanumeric();

        let expected_chars = ['a', '5', '中', '９', 'ñ', '٠'];

        for expected_ch in expected_chars {
            let (ch, new_cursor) = parser.parse(cursor).unwrap();
            assert_eq!(ch, expected_ch, "Failed in sequence for: {}", expected_ch);
            cursor = new_cursor;
        }
    }

    #[test]
    fn test_non_alphanumeric_fail() {
        let non_alphanumeric = [
            // Punctuation
            "!", ".", ",", ";", ":", "?", "'", "\"", // Symbols
            "@", "#", "$", "%", "&", "*", "+", "-", "=", // Whitespace
            " ", "\t", "\n", "\r", "\u{00A0}", "\u{2000}", // Emojis and symbols
            "🚀", "🦀", "💻", "♠", "♣", "€", "©", "®", // Brackets and delimiters
            "(", ")", "[", "]", "{", "}", "<", ">",
        ];

        for input in non_alphanumeric {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_alphanumeric();

            let result = parser.parse(cursor);
            assert!(
                result.is_err(),
                "Expected error for non-alphanumeric: {}",
                input
            );
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("expected Unicode letter or digit"),
                "Wrong error message for: {}",
                input
            );
        }
    }

    #[test]
    fn test_identifier_like_parsing() {
        // Test parsing something that looks like a programming identifier
        let input = "température123";
        let data = input.as_bytes();
        let mut cursor = ByteCursor::new(data);
        let parser = unicode_alphanumeric();

        let mut result = String::new();
        while !matches!(cursor, ByteCursor::EndOfFile { .. }) {
            match parser.parse(cursor) {
                Ok((ch, new_cursor)) => {
                    result.push(ch);
                    cursor = new_cursor;
                }
                Err(_) => break,
            }
        }

        assert_eq!(result, "température123");
    }

    #[test]
    fn test_stops_at_non_alphanumeric() {
        let input = "abc123.def";
        let data = input.as_bytes();
        let mut cursor = ByteCursor::new(data);
        let parser = unicode_alphanumeric();

        let mut result = String::new();
        while let Ok((ch, new_cursor)) = parser.parse(cursor) {
            result.push(ch);
            cursor = new_cursor;
        }

        assert_eq!(result, "abc123");

        // Should stop at the '.'
        let char_parser = char();
        let (next_ch, _) = char_parser.parse(cursor).unwrap();
        assert_eq!(next_ch, '.');
    }

    #[test]
    fn test_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = unicode_alphanumeric();

        let result = parser.parse(cursor);
        assert!(result.is_err(), "Expected error for empty input");
    }
}
