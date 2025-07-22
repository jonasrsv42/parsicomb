pub mod between;
pub mod separated_list;
pub mod separated_pair;

use crate::ParsicombError;
use crate::error::ErrorBranch;
use crate::filter::FilterExt;
use crate::parser::Parser;
use crate::utf8::char::char;

pub use between::between;
pub use separated_list::separated_list;
pub use separated_pair::separated_pair;

/// Convenience function to create a Unicode whitespace parser
pub fn unicode_whitespace<'a>()
-> impl Parser<'a, Output = char, Error: ErrorBranch<Base = ParsicombError<'a>>> {
    char().filter(|c| c.is_whitespace(), "expected Unicode whitespace")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_cursor::ByteCursor;

    #[test]
    fn test_ascii_whitespace() {
        let ascii_whitespace = [
            (" ", ' '),   // U+0020 SPACE
            ("\t", '\t'), // U+0009 TAB
            ("\n", '\n'), // U+000A LINE FEED
            ("\r", '\r'), // U+000D CARRIAGE RETURN
        ];

        for (input, expected) in ascii_whitespace {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_whitespace();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected,
                "Failed for ASCII whitespace: U+{:04X}",
                expected as u32
            );
        }
    }

    #[test]
    fn test_unicode_whitespace() {
        let test_cases = [
            // Unicode spaces
            ("\u{00A0}", '\u{00A0}'), // Non-breaking space
            ("\u{1680}", '\u{1680}'), // Ogham space mark
            ("\u{2000}", '\u{2000}'), // En quad
            ("\u{2001}", '\u{2001}'), // Em quad
            ("\u{2002}", '\u{2002}'), // En space
            ("\u{2003}", '\u{2003}'), // Em space
            ("\u{2004}", '\u{2004}'), // Three-per-em space
            ("\u{2005}", '\u{2005}'), // Four-per-em space
            ("\u{2006}", '\u{2006}'), // Six-per-em space
            ("\u{2007}", '\u{2007}'), // Figure space
            ("\u{2008}", '\u{2008}'), // Punctuation space
            ("\u{2009}", '\u{2009}'), // Thin space
            ("\u{200A}", '\u{200A}'), // Hair space
            ("\u{202F}", '\u{202F}'), // Narrow no-break space
            ("\u{205F}", '\u{205F}'), // Medium mathematical space
            ("\u{3000}", '\u{3000}'), // Ideographic space
            // Line separators
            ("\u{2028}", '\u{2028}'), // Line separator
            ("\u{2029}", '\u{2029}'), // Paragraph separator
            // Other whitespace
            ("\u{000B}", '\u{000B}'), // Vertical tab
            ("\u{000C}", '\u{000C}'), // Form feed
            ("\u{0085}", '\u{0085}'), // Next line
        ];

        for (input, expected) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_whitespace();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected,
                "Failed for Unicode whitespace: U+{:04X}",
                expected as u32
            );
        }
    }

    #[test]
    fn test_non_whitespace_fail() {
        let non_whitespace = ["a", "A", "0", "9", "!", ".", "ñ", "中", "🚀", "α", "Ω", "٠"];

        for input in non_whitespace {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_whitespace();

            let result = parser.parse(cursor);
            assert!(
                result.is_err(),
                "Expected error for non-whitespace: {}",
                input
            );
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("expected Unicode whitespace"),
                "Wrong error message for: {}",
                input
            );
        }
    }

    #[test]
    fn test_zero_width_spaces() {
        // Zero-width characters that are not considered whitespace by Rust
        let zero_width = [
            "\u{200B}", // Zero width space
            "\u{200C}", // Zero width non-joiner
            "\u{200D}", // Zero width joiner
            "\u{FEFF}", // Zero width no-break space (BOM)
        ];

        for input in zero_width {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_whitespace();

            let result = parser.parse(cursor);
            // These should fail as they're not considered whitespace by char::is_whitespace()
            assert!(
                result.is_err(),
                "Zero-width character should not be whitespace: U+{:04X}",
                input.chars().next().unwrap() as u32
            );
        }
    }

    #[test]
    fn test_cursor_advancement() {
        // Test that cursor advances correctly for multi-byte whitespace
        let input = "\u{3000}abc"; // Ideographic space (3 bytes) + "abc"
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = unicode_whitespace();

        let (ch, new_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, '\u{3000}');

        // Should be positioned at 'a'
        let char_parser = char();
        let (next_ch, _) = char_parser.parse(new_cursor).unwrap();
        assert_eq!(next_ch, 'a');
    }

    #[test]
    fn test_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = unicode_whitespace();

        let result = parser.parse(cursor);
        assert!(result.is_err(), "Expected error for empty input");
    }
}
