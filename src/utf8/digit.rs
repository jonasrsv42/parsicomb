use crate::parser::Parser;
use crate::utf8::char::char;
use crate::filter::FilterExt;

/// Convenience function to create a Unicode digit parser
pub fn unicode_digit() -> impl for<'a> Parser<'a, Output = char> {
    char().filter(
        |c| c.is_numeric(), 
        "expected Unicode digit"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_cursor::ByteCursor;

    #[test]
    fn test_ascii_digits() {
        for digit in '0'..='9' {
            let input = digit.to_string();
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data).unwrap();
            let parser = unicode_digit();
            
            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(ch, digit, "Failed for ASCII digit: {}", digit);
        }
    }

    #[test]
    fn test_unicode_digits() {
        let test_cases = [
            // Arabic-Indic digits
            ("٠", '٠'), // U+0660 Arabic-Indic digit zero
            ("١", '١'), // U+0661 Arabic-Indic digit one
            ("٩", '٩'), // U+0669 Arabic-Indic digit nine
            
            // Devanagari digits
            ("०", '०'), // U+0966 Devanagari digit zero
            ("१", '१'), // U+0967 Devanagari digit one
            ("९", '९'), // U+096F Devanagari digit nine
            
            // Fullwidth digits
            ("０", '０'), // U+FF10 Fullwidth digit zero
            ("５", '５'), // U+FF15 Fullwidth digit five
            ("９", '９'), // U+FF19 Fullwidth digit nine
        ];
        
        for (input, expected) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data).unwrap();
            let parser = unicode_digit();
            
            let (ch, _) = parser.parse(cursor).unwrap();
            assert_eq!(ch, expected, "Failed for Unicode digit: {} (U+{:04X})", input, expected as u32);
        }
    }

    #[test]
    fn test_non_digits_fail() {
        let non_digits = [
            "a", "A", "!", " ", "\t", ".", 
            "ñ", "中", "🚀", "α", "Ω"
        ];
        
        for input in non_digits {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data).unwrap();
            let parser = unicode_digit();
            
            let result = parser.parse(cursor);
            assert!(result.is_err(), "Expected error for non-digit: {}", input);
            assert!(result.unwrap_err().to_string().contains("expected Unicode digit"), 
                   "Wrong error message for: {}", input);
        }
    }

    #[test]
    fn test_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = unicode_digit();
        
        let result = parser.parse(cursor);
        assert!(result.is_err(), "Expected error for empty input");
    }
}
