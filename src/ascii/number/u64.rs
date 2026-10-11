//! Unsigned ASCII integers

use super::digit::digit;
use crate::ByteCursor;
use crate::ParsicombError;
use crate::cursor::Cursor;
use crate::parser::Parser;
use crate::parser_fn::parser_fn;
use crate::recognize::recognize;
use crate::skip_some::skip_some;

/// Parser that matches one or more ASCII digits and returns them as a u64
///
/// Fails at the first digit if the number does not fit in a u64.
pub fn u64<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = u64, Error = ParsicombError<'code>> {
    parser_fn(|start: ByteCursor<'code>| {
        let (digits, end) = recognize(skip_some(digit())).parse(start)?;

        let value = digits.slice().iter().try_fold(0u64, |value, digit| {
            value.checked_mul(10)?.checked_add(u64::from(digit - b'0'))
        });
        match value {
            Some(value) => Ok((value, end)),
            None => {
                Err(start.syntax_error(format!("number too large: {}", digits.as_string()).into()))
            }
        }
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
}
