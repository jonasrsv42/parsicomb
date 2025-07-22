use crate::byte::between_bytes;
use crate::parser::Parser;
use crate::ParsicombError;

/// Parser that matches a single ASCII digit (0-9)
pub fn digit<'code>() -> impl Parser<'code, Output = u8, Error = ParsicombError<'code>> {
    between_bytes(b'0', b'9')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_cursor::ByteCursor;

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
        assert!(result.unwrap_err().to_string().contains("expected byte"));
    }

    #[test]
    fn test_digit_letter_fails() {
        let data = b"a123";
        let cursor = ByteCursor::new(data);
        let parser = digit();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }
}
