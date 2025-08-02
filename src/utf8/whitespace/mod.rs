//! # Unicode Whitespace Support
//!
//! This module provides basic Unicode whitespace parsing functionality.
//!
//! ## Why No Generic Whitespace Combinators?
//!
//! You might expect to find generic whitespace-aware combinators here like
//! `whitespace_between()`, `whitespace_separated_list()`, etc. We intentionally
//! don't provide these because they create two significant problems:
//!
//! ### 1. Poor Error Location Reporting
//!
//! Generic whitespace combinators report errors at the wrong location:
//!
//! ```text
//! Input: "type hello    world"
//!                      ^ Error points here after consuming whitespace
//! ```
//!
//! But users expect errors to point to meaningful locations:
//!
//! ```text
//! Input: "type hello    world"  
//!                  ^ Error should point here (after "hello")
//! ```
//!
//! ### 2. Generic Error Messages
//!
//! Generic combinators can only provide generic error messages:
//! - ❌ "Expected content after whitespace"
//! - ❌ "Parser failed"
//!
//! But good parsers provide semantic, contextual error messages:
//! - ✅ "Expected semicolon after statement"
//! - ✅ "Missing closing bracket"
//! - ✅ "Expected type annotation"
//!
//! ## Recommended Approach
//!
//! Instead of generic whitespace combinators, create semantic combinators
//! that understand the parsing context:
//!
//! ```text
//! ❌ Don't do this:
//! whitespace_separated_pair(type_parser(), is_string(","), value_parser())
//!
//! ✅ Do this instead:
//! fn parameter_declaration() -> impl Parser<...> {
//!     // Handle whitespace and provide semantic errors
//! }
//! ```
//!
//! This approach gives you:
//! - Precise error locations (before whitespace consumption)
//! - Meaningful error messages ("Expected parameter name", not "Parser failed")
//! - Full control over whitespace handling for your specific syntax

use crate::ByteCursor;
use crate::ParsicombError;
use crate::filter::{FilterError, FilterExt};
use crate::parser::Parser;
use crate::utf8::char::char;

/// Convenience function to create a Unicode whitespace parser
pub fn unicode_whitespace<'a>()
-> impl Parser<'a, Cursor = ByteCursor<'a>, Output = char, Error = FilterError<'a, ParsicombError<'a>>>
{
    char().filter(|c| c.is_whitespace(), "expected Unicode whitespace")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;

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
