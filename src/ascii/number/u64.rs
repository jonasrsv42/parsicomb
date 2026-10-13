//! Unsigned ASCII integers

use super::{digits, unsigned_value};
use crate::ByteCursor;
use crate::ParsicombError;
use crate::combinator::parser_fn;
use crate::combinator::recognize;
use crate::parser::Parser;

/// Parser that matches one or more ASCII digits and returns them as a u64
///
/// Fails after the digits if the number does not fit in a u64.
pub fn u64<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = u64, Error = ParsicombError<'code>> {
    parser_fn(|start: ByteCursor<'code>| {
        let (digits, end) = recognize(digits()).parse(start)?;
        Ok((unsigned_value(end, digits.slice())?, end))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;

    #[test]
    fn test_uint_single_digit() {
        let data = b"5abc";
        let cursor = ByteCursor::new(data);
        let parser = u64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 5);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_uint_multiple_digits() {
        let data = b"123abc";
        let cursor = ByteCursor::new(data);
        let parser = u64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 123);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_uint_zero() {
        let data = b"0";
        let cursor = ByteCursor::new(data);
        let parser = u64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 0);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_uint_large_number() {
        let data = b"9876543210";
        let cursor = ByteCursor::new(data);
        let parser = u64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 9876543210);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_uint_no_digit_fails() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = u64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_uint_stops_at_non_digit() {
        let data = b"42.5";
        let cursor = ByteCursor::new(data);
        let parser = u64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 42);
        assert_eq!(cursor.value().unwrap(), b'.');
    }

    #[test]
    fn test_uint_overflow() {
        // This number is larger than u64::MAX
        let data = b"99999999999999999999999999999999";
        let cursor = ByteCursor::new(data);
        let parser = u64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("number too large"));
    }

    #[test]
    fn test_overflow_message_is_bounded() {
        let input = vec![b'9'; 1 << 20];
        let message = u64()
            .parse(ByteCursor::new(&input))
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("number too large: 999999999999999999999999… (1048576 digits)"),
            "got {}",
            &message[..200.min(message.len())]
        );
        assert!(message.len() < 5_000, "got {} bytes", message.len());
    }
}
