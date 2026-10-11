//! ASCII digits

use crate::ByteCursor;
use crate::ParsicombError;
use crate::byte::byte;
use crate::filter::FilterExt;
use crate::map_err::MapErrExt;
use crate::parser::Parser;

/// Parser that matches a single ASCII digit (0-9); fails with
/// `expected a digit, found …`
pub fn digit<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = u8, Error = ParsicombError<'code>> {
    byte()
        .filter(u8::is_ascii_digit, "a digit")
        .map_err(ParsicombError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;

    #[test]
    fn test_digit_zero() {
        let data = b"0abc";
        let cursor = ByteCursor::new(data);
        let parser = digit();

        let (d, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(d, b'0');
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_digit_nine() {
        let data = b"9xyz";
        let cursor = ByteCursor::new(data);
        let parser = digit();

        let (d, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(d, b'9');
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_digit_middle() {
        let data = b"5";
        let cursor = ByteCursor::new(data);
        let parser = digit();

        let (d, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(d, b'5');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_digit_non_digit_fails() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = digit();

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected a digit"));
    }

    #[test]
    fn test_digit_letter_fails() {
        let data = b"a123";
        let cursor = ByteCursor::new(data);
        let parser = digit();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_failure_says_a_digit() {
        let message = digit()
            .parse(ByteCursor::new(b"x"))
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("expected a digit, found 'x'"),
            "got {message}"
        );
    }
}
