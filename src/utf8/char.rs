use crate::ByteCursor;
use crate::Cursor;
use crate::byte::ByteParser;
use crate::parser::Parser;
use crate::{CodeLoc, ParsicombError};
use std::borrow::Cow;

/// Parser that consumes and returns a single UTF-8 character
pub struct CharParser;

// Helper function to reduce error creation boilerplate
fn create_error<'code>(
    cursor: &ByteCursor<'code>,
    message: Cow<'static, str>,
) -> ParsicombError<'code> {
    let (data, position) = cursor.inner();
    ParsicombError::SyntaxError {
        message,
        loc: CodeLoc::new(data, position),
    }
}

impl<'code> Parser<'code> for CharParser {
    type Cursor = ByteCursor<'code>;
    type Output = char;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let byte_parser = ByteParser::new();

        // 1. Read the first byte
        let (b1, mut current_cursor) = byte_parser.parse(cursor)?;

        // 2. Decode based on the first byte
        let codepoint = if b1 < 0x80 {
            // ASCII fast path
            return Ok((b1 as char, current_cursor));
        } else if b1 < 0xC0 {
            // Continuation byte used as start byte (0x80-0xBF)
            return Err(create_error(&cursor, "invalid UTF-8 start byte".into()));
        } else if b1 < 0xE0 {
            // 2-byte sequence: 110xxxxx 10xxxxxx
            let (b2, new_cursor) = byte_parser
                .parse(current_cursor)
                .map_err(|_| create_error(&current_cursor, "incomplete UTF-8 sequence".into()))?;
            current_cursor = new_cursor;

            if (b2 & 0xC0) != 0x80 {
                return Err(create_error(
                    &current_cursor,
                    "invalid UTF-8 continuation byte".into(),
                ));
            }

            let cp = ((b1 as u32 & 0x1F) << 6) | (b2 as u32 & 0x3F);
            if cp < 0x80 {
                return Err(create_error(&cursor, "overlong UTF-8 encoding".into()));
            }
            cp
        } else if b1 < 0xF0 {
            // 3-byte sequence: 1110xxxx 10xxxxxx 10xxxxxx
            let (b2, c2) = byte_parser
                .parse(current_cursor)
                .map_err(|_| create_error(&current_cursor, "incomplete UTF-8 sequence".into()))?;
            let (b3, c3) = byte_parser
                .parse(c2)
                .map_err(|_| create_error(&c2, "incomplete UTF-8 sequence".into()))?;
            current_cursor = c3;

            if (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80 {
                return Err(create_error(
                    &current_cursor,
                    "invalid UTF-8 continuation byte".into(),
                ));
            }

            let cp = ((b1 as u32 & 0x0F) << 12) | ((b2 as u32 & 0x3F) << 6) | (b3 as u32 & 0x3F);
            if cp < 0x800 {
                return Err(create_error(&cursor, "overlong UTF-8 encoding".into()));
            }
            if (0xD800..=0xDFFF).contains(&cp) {
                return Err(create_error(&cursor, "UTF-16 surrogate in UTF-8".into()));
            }
            cp
        } else if b1 < 0xF8 {
            // 4-byte sequence: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
            let (b2, c2) = byte_parser
                .parse(current_cursor)
                .map_err(|_| create_error(&current_cursor, "incomplete UTF-8 sequence".into()))?;
            let (b3, c3) = byte_parser
                .parse(c2)
                .map_err(|_| create_error(&c2, "incomplete UTF-8 sequence".into()))?;
            let (b4, c4) = byte_parser
                .parse(c3)
                .map_err(|_| create_error(&c3, "incomplete UTF-8 sequence".into()))?;
            current_cursor = c4;

            if (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80 || (b4 & 0xC0) != 0x80 {
                return Err(create_error(
                    &current_cursor,
                    "invalid UTF-8 continuation byte".into(),
                ));
            }

            let cp = ((b1 as u32 & 0x07) << 18)
                | ((b2 as u32 & 0x3F) << 12)
                | ((b3 as u32 & 0x3F) << 6)
                | (b4 as u32 & 0x3F);
            if cp < 0x10000 {
                return Err(create_error(&cursor, "overlong UTF-8 encoding".into()));
            }
            if cp > 0x10FFFF {
                return Err(create_error(
                    &cursor,
                    "codepoint beyond Unicode range".into(),
                ));
            }
            cp
        } else {
            // Invalid start byte
            return Err(create_error(&cursor, "invalid UTF-8 start byte".into()));
        };

        // 3. Convert final codepoint to char
        let ch = char::from_u32(codepoint).ok_or_else(|| {
            create_error(
                &cursor,
                format!("invalid Unicode codepoint: U+{:04X}", codepoint).into(),
            )
        })?;

        Ok((ch, current_cursor))
    }
}

/// Convenience function to create a CharParser
pub fn char() -> CharParser {
    CharParser
}

/// Parser that matches a specific character
pub struct IsChar(char);

impl<'code> Parser<'code> for IsChar {
    type Cursor = ByteCursor<'code>;
    type Output = char;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (ch, next_cursor) = char().parse(cursor)?;
        if ch == self.0 {
            Ok((ch, next_cursor))
        } else {
            Err(create_error(
                &cursor,
                format!("expected '{}', found '{}'", self.0, ch).into(),
            ))
        }
    }
}

/// Convenience function to create a parser that matches a specific character
pub fn is_char(expected: char) -> IsChar {
    IsChar(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_char() {
        let data = "hello".as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = char();

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'h');

        let (ch, _) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'e');
    }

    #[test]
    fn test_unicode_chars() {
        // Swedish characters (2-byte UTF-8)
        let data = "åäö".as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = char();

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'å');

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'ä');

        let (ch, _) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'ö');
    }

    #[test]
    fn test_emoji() {
        // Emoji (4-byte UTF-8)
        let data = "🦀".as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = char();

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, '🦀');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_mixed_chars() {
        let data = "café🦀".as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = char();

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'c');

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'a');

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'f');

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'é');

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, '🦀');

        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_invalid_utf8() {
        let data = &[0xFF, 0xFE];
        let cursor = ByteCursor::new(data);
        let parser = char();

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid UTF-8"));
    }

    #[test]
    fn test_incomplete_sequence() {
        // Start of 2-byte sequence but missing second byte
        let data = &[0xC3]; // Start of "ä" but incomplete
        let cursor = ByteCursor::new(data);
        let parser = char();

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("incomplete UTF-8"));
    }

    #[test]
    fn test_1_byte_chars_comprehensive() {
        // Test ASCII range boundaries
        let test_cases = [
            (0x00, '\x00'), // NULL
            (0x20, ' '),    // SPACE
            (0x41, 'A'),    // LATIN CAPITAL LETTER A
            (0x61, 'a'),    // LATIN SMALL LETTER A
            (0x7F, '\x7F'), // DEL
        ];

        for (byte_val, expected_char) in test_cases {
            let data = &[byte_val];
            let cursor = ByteCursor::new(data);
            let parser = char();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(ch, expected_char, "Failed for byte 0x{:02X}", byte_val);
        }
    }

    #[test]
    fn test_2_byte_chars_comprehensive() {
        // Test various 2-byte UTF-8 characters
        let test_cases = [
            ("Ä", 'Ä'), // U+00C4 LATIN CAPITAL LETTER A WITH DIAERESIS
            ("ä", 'ä'), // U+00E4 LATIN SMALL LETTER A WITH DIAERESIS
            ("ñ", 'ñ'), // U+00F1 LATIN SMALL LETTER N WITH TILDE
            ("ß", 'ß'), // U+00DF LATIN SMALL LETTER SHARP S
            ("Ω", 'Ω'), // U+03A9 GREEK CAPITAL LETTER OMEGA
            ("α", 'α'), // U+03B1 GREEK SMALL LETTER ALPHA
            ("Я", 'Я'), // U+042F CYRILLIC CAPITAL LETTER YA
            ("я", 'я'), // U+044F CYRILLIC SMALL LETTER YA
        ];

        for (input_str, expected_char) in test_cases {
            let data = input_str.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = char();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(ch, expected_char, "Failed for input: {}", input_str);

            // Verify it's actually 2 bytes
            assert_eq!(
                input_str.len(),
                2,
                "Expected 2-byte sequence for {}",
                input_str
            );
        }
    }

    #[test]
    fn test_3_byte_chars_comprehensive() {
        // Test various 3-byte UTF-8 characters
        let test_cases = [
            ("€", '€'),   // U+20AC EURO SIGN
            ("中", '中'), // U+4E2D CJK UNIFIED IDEOGRAPH (Chinese)
            ("文", '文'), // U+6587 CJK UNIFIED IDEOGRAPH (Chinese)
            ("字", '字'), // U+5B57 CJK UNIFIED IDEOGRAPH (Chinese)
            ("あ", 'あ'), // U+3042 HIRAGANA LETTER A (Japanese)
            ("か", 'か'), // U+304B HIRAGANA LETTER KA (Japanese)
            ("가", '가'), // U+AC00 HANGUL SYLLABLE GA (Korean)
            ("나", '나'), // U+B098 HANGUL SYLLABLE NA (Korean)
            ("♠", '♠'),   // U+2660 BLACK SPADE SUIT
            ("♣", '♣'),   // U+2663 BLACK CLUB SUIT
        ];

        for (input_str, expected_char) in test_cases {
            let data = input_str.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = char();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected_char,
                "Failed for input: {} (U+{:04X})",
                input_str, expected_char as u32
            );

            // Verify it's actually 3 bytes
            assert_eq!(
                input_str.len(),
                3,
                "Expected 3-byte sequence for {}",
                input_str
            );
        }
    }

    #[test]
    fn test_4_byte_chars_comprehensive() {
        // Test various 4-byte UTF-8 characters (emojis and supplementary characters)
        let test_cases = [
            ("🦀", '🦀'), // U+1F980 CRAB
            ("🚀", '🚀'), // U+1F680 ROCKET
            ("🎉", '🎉'), // U+1F389 PARTY POPPER
            ("🌟", '🌟'), // U+1F31F GLOWING STAR
            ("💻", '💻'), // U+1F4BB PERSONAL COMPUTER
            ("🔥", '🔥'), // U+1F525 FIRE
            ("🌊", '🌊'), // U+1F30A WATER WAVE
            ("🎯", '🎯'), // U+1F3AF DIRECT HIT
            ("🎨", '🎨'), // U+1F3A8 ARTIST PALETTE
            ("🌈", '🌈'), // U+1F308 RAINBOW
        ];

        for (input_str, expected_char) in test_cases {
            let data = input_str.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = char();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected_char,
                "Failed for input: {} (U+{:04X})",
                input_str, expected_char as u32
            );

            // Verify it's actually 4 bytes
            assert_eq!(
                input_str.len(),
                4,
                "Expected 4-byte sequence for {}",
                input_str
            );
        }
    }

    #[test]
    fn test_invalid_2_byte_sequences() {
        // Invalid 2-byte sequences
        let invalid_sequences = [
            &[0xC0, 0x80][..], // Overlong encoding of NULL
            &[0xC1, 0xBF][..], // Overlong encoding
            &[0xC2, 0x00][..], // Invalid continuation byte
            &[0xC2, 0xFF][..], // Invalid continuation byte
        ];

        for seq in invalid_sequences {
            let cursor = ByteCursor::new(seq);
            let parser = char();
            let result = parser.parse(cursor);
            assert!(
                result.is_err(),
                "Expected error for invalid sequence: {:?}",
                seq
            );
        }
    }

    #[test]
    fn test_invalid_3_byte_sequences() {
        // Invalid 3-byte sequences
        let invalid_sequences = [
            &[0xE0, 0x80, 0x80][..], // Overlong encoding
            &[0xE0, 0x9F, 0xBF][..], // Overlong encoding
            &[0xED, 0xA0, 0x80][..], // UTF-16 surrogate
            &[0xED, 0xBF, 0xBF][..], // UTF-16 surrogate
        ];

        for seq in invalid_sequences {
            let cursor = ByteCursor::new(seq);
            let parser = char();
            let result = parser.parse(cursor);
            assert!(
                result.is_err(),
                "Expected error for invalid sequence: {:?}",
                seq
            );
        }
    }

    #[test]
    fn test_edge_case_codepoints() {
        // Test edge cases at Unicode boundaries
        let test_cases = [
            ("\u{0080}", '\u{0080}'),     // First 2-byte character
            ("\u{07FF}", '\u{07FF}'),     // Last 2-byte character
            ("\u{0800}", '\u{0800}'),     // First 3-byte character
            ("\u{FFFF}", '\u{FFFF}'),     // Last 3-byte character (excluding surrogates)
            ("\u{10000}", '\u{10000}'),   // First 4-byte character
            ("\u{10FFFF}", '\u{10FFFF}'), // Last valid Unicode character
        ];

        for (input_str, expected_char) in test_cases {
            let data = input_str.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = char();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected_char,
                "Failed for edge case: U+{:04X}",
                expected_char as u32
            );
        }
    }

    #[test]
    fn test_invalid_start_bytes() {
        // Test invalid UTF-8 start bytes
        let invalid_start_bytes = [
            0x80, 0x81, 0x82, 0x8F, // Continuation bytes used as start
            0x90, 0x9F, 0xA0, 0xAF, // More continuation bytes
            0xB0, 0xBF, // More continuation bytes
            0xF8, 0xF9, 0xFA, 0xFB, // Invalid 5-byte start
            0xFC, 0xFD, // Invalid 6-byte start
            0xFE, 0xFF, // Invalid bytes
        ];

        for byte in invalid_start_bytes {
            let data = &[byte, 0x80]; // Add a continuation byte
            let cursor = ByteCursor::new(data);
            let parser = char();
            let result = parser.parse(cursor);
            assert!(
                result.is_err(),
                "Expected error for invalid start byte: 0x{:02X}",
                byte
            );
        }
    }

    #[test]
    fn test_truncated_sequences() {
        // Test truncated multi-byte sequences at different positions
        let truncated_cases = [
            // 2-byte sequences missing 2nd byte
            (&[0xC2][..], "2-byte missing continuation"),
            (&[0xDF][..], "2-byte missing continuation"),
            // 3-byte sequences missing bytes
            (&[0xE0][..], "3-byte missing all continuations"),
            (&[0xE0, 0xA0][..], "3-byte missing last continuation"),
            (&[0xEF][..], "3-byte missing all continuations"),
            (&[0xEF, 0xBF][..], "3-byte missing last continuation"),
            // 4-byte sequences missing bytes
            (&[0xF0][..], "4-byte missing all continuations"),
            (&[0xF0, 0x90][..], "4-byte missing 2 continuations"),
            (&[0xF0, 0x90, 0x80][..], "4-byte missing last continuation"),
            (&[0xF4][..], "4-byte missing all continuations"),
            (&[0xF4, 0x8F][..], "4-byte missing 2 continuations"),
            (&[0xF4, 0x8F, 0xBF][..], "4-byte missing last continuation"),
        ];

        for (data, description) in truncated_cases {
            let cursor = ByteCursor::new(data);
            let parser = char();
            let result = parser.parse(cursor);
            assert!(result.is_err(), "Expected error for {}", description);
            assert!(
                result.unwrap_err().to_string().contains("incomplete UTF-8"),
                "Expected 'incomplete UTF-8' error for {}",
                description
            );
        }
    }

    #[test]
    fn test_mixed_valid_invalid_continuation_bytes() {
        // Test sequences with some valid and some invalid continuation bytes
        let mixed_invalid_cases = [
            // 2-byte with invalid 2nd byte
            (&[0xC2, 0x00][..], "2-byte invalid continuation"),
            (&[0xC2, 0x40][..], "2-byte invalid continuation"),
            (&[0xC2, 0xC0][..], "2-byte invalid continuation"),
            // 3-byte with invalid 2nd byte
            (&[0xE0, 0x00, 0x80][..], "3-byte invalid 2nd byte"),
            (&[0xE0, 0xA0, 0x00][..], "3-byte invalid 3rd byte"),
            (&[0xE0, 0x40, 0x80][..], "3-byte invalid 2nd byte"),
            // 4-byte with mixed invalid bytes
            (&[0xF0, 0x00, 0x80, 0x80][..], "4-byte invalid 2nd byte"),
            (&[0xF0, 0x90, 0x00, 0x80][..], "4-byte invalid 3rd byte"),
            (&[0xF0, 0x90, 0x80, 0x00][..], "4-byte invalid 4th byte"),
        ];

        for (data, description) in mixed_invalid_cases {
            let cursor = ByteCursor::new(data);
            let parser = char();
            let result = parser.parse(cursor);
            assert!(result.is_err(), "Expected error for {}", description);
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("continuation byte"),
                "Expected 'continuation byte' error for {}",
                description
            );
        }
    }

    #[test]
    fn test_beyond_unicode_range() {
        // Test 4-byte sequences that encode values > U+10FFFF
        let beyond_unicode = [
            &[0xF4, 0x90, 0x80, 0x80][..], // U+110000 (first invalid)
            &[0xF5, 0x80, 0x80, 0x80][..], // Way beyond Unicode
            &[0xF7, 0xBF, 0xBF, 0xBF][..], // Maximum 4-byte value
        ];

        for data in beyond_unicode {
            let cursor = ByteCursor::new(data);
            let parser = char();
            let result = parser.parse(cursor);
            assert!(
                result.is_err(),
                "Expected error for beyond Unicode: {:?}",
                data
            );
            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains("beyond Unicode") || error_msg.contains("invalid UTF-8"),
                "Expected Unicode range error for {:?}",
                data
            );
        }
    }

    #[test]
    fn test_surrogate_pairs_comprehensive() {
        // Test the full range of UTF-16 surrogates that should be invalid in UTF-8
        let surrogate_ranges = [
            0xD800, 0xD801, 0xD8FF, // High surrogates start
            0xDB00, 0xDBFF, // High surrogates end
            0xDC00, 0xDC01, 0xDCFF, // Low surrogates start
            0xDF00, 0xDFFF, // Low surrogates end
        ];

        for codepoint in surrogate_ranges {
            // Manually encode as 3-byte UTF-8 (which would be invalid)
            let bytes = [
                0xE0 | ((codepoint >> 12) & 0x0F) as u8,
                0x80 | ((codepoint >> 6) & 0x3F) as u8,
                0x80 | (codepoint & 0x3F) as u8,
            ];

            let cursor = ByteCursor::new(&bytes);
            let parser = char();
            let result = parser.parse(cursor);
            assert!(
                result.is_err(),
                "Expected error for surrogate U+{:04X}",
                codepoint
            );
            assert!(
                result.unwrap_err().to_string().contains("surrogate"),
                "Expected surrogate error for U+{:04X}",
                codepoint
            );
        }
    }

    #[test]
    fn test_modified_utf8_compatibility() {
        // Test that we reject "Modified UTF-8" encoding of NULL
        // Modified UTF-8 encodes NULL as 0xC0 0x80 instead of 0x00
        let modified_utf8_null = &[0xC0, 0x80];
        let cursor = ByteCursor::new(modified_utf8_null);
        let parser = char();
        let result = parser.parse(cursor);
        assert!(
            result.is_err(),
            "Should reject Modified UTF-8 NULL encoding"
        );
        assert!(
            result.unwrap_err().to_string().contains("overlong"),
            "Should detect overlong encoding"
        );
    }

    #[test]
    fn test_cursor_advancement_accuracy() {
        // Test that cursor advances by exactly the right number of bytes
        let test_string = "A𝟘🦀"; // 1-byte + 4-byte + 4-byte
        let data = test_string.as_bytes();
        let mut cursor = ByteCursor::new(data);
        let parser = char();

        // Parse 'A' (1 byte)
        let (ch, new_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'A');
        cursor = new_cursor;

        // Parse '𝟘' (4 bytes) - mathematical bold digit zero
        let (ch, new_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, '𝟘');
        cursor = new_cursor;

        // Parse '🦀' (4 bytes)
        let (ch, new_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, '🦀');
        cursor = new_cursor;

        // Should be at end
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_null_and_control_chars() {
        // Test that we correctly handle NULL and control characters
        let control_chars = [
            (0x00, '\x00'), // NULL
            (0x01, '\x01'), // SOH
            (0x08, '\x08'), // BACKSPACE
            (0x09, '\x09'), // TAB
            (0x0A, '\x0A'), // LINE FEED
            (0x0D, '\x0D'), // CARRIAGE RETURN
            (0x1F, '\x1F'), // UNIT SEPARATOR
            (0x7F, '\x7F'), // DELETE
        ];

        for (byte_val, expected_char) in control_chars {
            let data = &[byte_val];
            let cursor = ByteCursor::new(data);
            let parser = char();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected_char,
                "Failed for control char 0x{:02X}",
                byte_val
            );
        }
    }
}
