//! Unicode letters

use crate::ByteCursor;
use crate::ParsicombError;
use crate::combinator::FilterExt;
use crate::combinator::MapErrExt;
use crate::parser::Parser;
use crate::utf8::char::char;

/// Parser that matches one Unicode letter ([`char::is_alphabetic`])
pub fn unicode_letter<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = char, Error = ParsicombError<'code>> {
    char()
        .filter(|c| c.is_alphabetic(), "a Unicode letter")
        .map_err(ParsicombError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_letters() {
        // Test all ASCII letters
        for letter in 'a'..='z' {
            let input = letter.to_string();
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_letter();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(ch, letter, "Failed for ASCII lowercase: {}", letter);
        }

        for letter in 'A'..='Z' {
            let input = letter.to_string();
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_letter();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(ch, letter, "Failed for ASCII uppercase: {}", letter);
        }
    }

    #[test]
    fn test_latin_extended_letters() {
        let test_cases = [
            // Latin Extended-A
            ("À", 'À'),
            ("Á", 'Á'),
            ("Â", 'Â'),
            ("Ã", 'Ã'),
            ("Ä", 'Ä'),
            ("Å", 'Å'),
            ("à", 'à'),
            ("á", 'á'),
            ("â", 'â'),
            ("ã", 'ã'),
            ("ä", 'ä'),
            ("å", 'å'),
            ("Ç", 'Ç'),
            ("ç", 'ç'),
            ("Ñ", 'Ñ'),
            ("ñ", 'ñ'),
            ("Ö", 'Ö'),
            ("ö", 'ö'),
            ("Ü", 'Ü'),
            ("ü", 'ü'),
            ("ß", 'ß'),
            // Nordic letters
            ("Æ", 'Æ'),
            ("æ", 'æ'),
            ("Ø", 'Ø'),
            ("ø", 'ø'),
            // Eastern European
            ("Ž", 'Ž'),
            ("ž", 'ž'),
            ("Š", 'Š'),
            ("š", 'š'),
        ];

        for (input, expected) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_letter();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected,
                "Failed for Latin extended: {} (U+{:04X})",
                input, expected as u32
            );
        }
    }

    #[test]
    fn test_greek_letters() {
        let test_cases = [
            // Greek uppercase
            ("Α", 'Α'),
            ("Β", 'Β'),
            ("Γ", 'Γ'),
            ("Δ", 'Δ'),
            ("Ε", 'Ε'),
            ("Ζ", 'Ζ'),
            ("Η", 'Η'),
            ("Θ", 'Θ'),
            ("Ι", 'Ι'),
            ("Κ", 'Κ'),
            ("Λ", 'Λ'),
            ("Μ", 'Μ'),
            ("Ν", 'Ν'),
            ("Ξ", 'Ξ'),
            ("Ο", 'Ο'),
            ("Π", 'Π'),
            ("Ρ", 'Ρ'),
            ("Σ", 'Σ'),
            ("Τ", 'Τ'),
            ("Υ", 'Υ'),
            ("Φ", 'Φ'),
            ("Χ", 'Χ'),
            ("Ψ", 'Ψ'),
            ("Ω", 'Ω'),
            // Greek lowercase
            ("α", 'α'),
            ("β", 'β'),
            ("γ", 'γ'),
            ("δ", 'δ'),
            ("ε", 'ε'),
            ("ζ", 'ζ'),
            ("η", 'η'),
            ("θ", 'θ'),
            ("ι", 'ι'),
            ("κ", 'κ'),
            ("λ", 'λ'),
            ("μ", 'μ'),
            ("ν", 'ν'),
            ("ξ", 'ξ'),
            ("ο", 'ο'),
            ("π", 'π'),
            ("ρ", 'ρ'),
            ("σ", 'σ'),
            ("τ", 'τ'),
            ("υ", 'υ'),
            ("φ", 'φ'),
            ("χ", 'χ'),
            ("ψ", 'ψ'),
            ("ω", 'ω'),
        ];

        for (input, expected) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_letter();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected,
                "Failed for Greek letter: {} (U+{:04X})",
                input, expected as u32
            );
        }
    }

    #[test]
    fn test_cyrillic_letters() {
        let test_cases = [
            // Cyrillic uppercase
            ("А", 'А'),
            ("Б", 'Б'),
            ("В", 'В'),
            ("Г", 'Г'),
            ("Д", 'Д'),
            ("Е", 'Е'),
            ("Ё", 'Ё'),
            ("Ж", 'Ж'),
            ("З", 'З'),
            ("И", 'И'),
            ("Й", 'Й'),
            ("К", 'К'),
            ("Л", 'Л'),
            ("М", 'М'),
            ("Н", 'Н'),
            ("О", 'О'),
            ("П", 'П'),
            ("Р", 'Р'),
            ("С", 'С'),
            ("Т", 'Т'),
            ("У", 'У'),
            ("Ф", 'Ф'),
            ("Х", 'Х'),
            ("Ц", 'Ц'),
            ("Ч", 'Ч'),
            ("Ш", 'Ш'),
            ("Щ", 'Щ'),
            ("Ъ", 'Ъ'),
            ("Ы", 'Ы'),
            ("Ь", 'Ь'),
            ("Э", 'Э'),
            ("Ю", 'Ю'),
            ("Я", 'Я'),
            // Cyrillic lowercase
            ("а", 'а'),
            ("б", 'б'),
            ("в", 'в'),
            ("г", 'г'),
            ("д", 'д'),
            ("е", 'е'),
            ("ё", 'ё'),
            ("ж", 'ж'),
            ("з", 'з'),
            ("и", 'и'),
            ("й", 'й'),
            ("к", 'к'),
            ("л", 'л'),
            ("м", 'м'),
            ("н", 'н'),
            ("о", 'о'),
            ("п", 'п'),
            ("р", 'р'),
            ("с", 'с'),
            ("т", 'т'),
            ("у", 'у'),
            ("ф", 'ф'),
            ("х", 'х'),
            ("ц", 'ц'),
            ("ч", 'ч'),
            ("ш", 'ш'),
            ("щ", 'щ'),
            ("ъ", 'ъ'),
            ("ы", 'ы'),
            ("ь", 'ь'),
            ("э", 'э'),
            ("ю", 'ю'),
            ("я", 'я'),
        ];

        for (input, expected) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_letter();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected,
                "Failed for Cyrillic letter: {} (U+{:04X})",
                input, expected as u32
            );
        }
    }

    #[test]
    fn test_cjk_letters() {
        let test_cases = [
            // Chinese (CJK Unified Ideographs)
            ("中", '中'),
            ("文", '文'),
            ("字", '字'),
            ("国", '国'),
            ("人", '人'),
            // Japanese Hiragana
            ("あ", 'あ'),
            ("か", 'か'),
            ("さ", 'さ'),
            ("た", 'た'),
            ("な", 'な'),
            ("は", 'は'),
            ("ま", 'ま'),
            ("や", 'や'),
            ("ら", 'ら'),
            ("わ", 'わ'),
            // Japanese Katakana
            ("ア", 'ア'),
            ("カ", 'カ'),
            ("サ", 'サ'),
            ("タ", 'タ'),
            ("ナ", 'ナ'),
            ("ハ", 'ハ'),
            ("マ", 'マ'),
            ("ヤ", 'ヤ'),
            ("ラ", 'ラ'),
            ("ワ", 'ワ'),
            // Korean Hangul
            ("가", '가'),
            ("나", '나'),
            ("다", '다'),
            ("라", '라'),
            ("마", '마'),
            ("바", '바'),
            ("사", '사'),
            ("아", '아'),
            ("자", '자'),
            ("차", '차'),
        ];

        for (input, expected) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_letter();

            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(
                ch, expected,
                "Failed for CJK letter: {} (U+{:04X})",
                input, expected as u32
            );
        }
    }

    #[test]
    fn test_non_letters_fail() {
        let non_letters = [
            // Digits
            "0", "1", "9", "٠", "५", "０", // Punctuation
            "!", ".", ",", ";", ":", "?", "'", "\"", // Symbols
            "@", "#", "$", "%", "&", "*", "+", "-", "=", // Whitespace
            " ", "\t", "\n", "\r", // Emojis and symbols
            "🚀", "🦀", "💻", "♠", "♣", "€", "©", "®",
        ];

        for input in non_letters {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);
            let parser = unicode_letter();

            let result = parser.parse(cursor);
            assert!(result.is_err(), "Expected error for non-letter: {}", input);
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("expected a Unicode letter"),
                "Wrong error message for: {}",
                input
            );
        }
    }

    #[test]
    fn test_cursor_advancement() {
        // Test that cursor advances correctly for multi-byte letters
        let input = "中文abc"; // Chinese characters (3 bytes each) + ASCII
        let data = input.as_bytes();
        let mut cursor = ByteCursor::new(data);
        let parser = unicode_letter();

        // Parse '中'
        let (ch, new_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, '中');
        cursor = new_cursor;

        // Parse '文'
        let (ch, new_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, '文');
        cursor = new_cursor;

        // Parse 'a'
        let (ch, new_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'a');
        cursor = new_cursor;

        // Parse 'b'
        let (ch, new_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'b');
        cursor = new_cursor;

        // Parse 'c'
        let (ch, _) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'c');
    }

    #[test]
    fn test_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = unicode_letter();

        let result = parser.parse(cursor);
        assert!(result.is_err(), "Expected error for empty input");
    }
}
