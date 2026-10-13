//! Signed ASCII integers

use super::u64::u64;
use super::{sign, signed_value};
use crate::ByteCursor;
use crate::ParsicombError;
use crate::optional::optional;
use crate::parser::Parser;
use crate::parser_fn::parser_fn;

/// Parser that matches ASCII integer numbers (positive or negative)
///
/// Fails after the number if it does not fit in an i64.
pub fn i64<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = i64, Error = ParsicombError<'code>> {
    parser_fn(|start: ByteCursor<'code>| {
        let (sign, cursor) = optional(sign()).parse(start)?;
        let (magnitude, cursor) = u64().parse(cursor)?;
        Ok((signed_value(cursor, sign, magnitude)?, cursor))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cursor;

    #[test]
    fn test_positive_integer() {
        let data = b"123abc";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 123);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_negative_integer() {
        let data = b"-456xyz";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, -456);
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_integer_with_plus() {
        let data = b"+789";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 789);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_single_digit() {
        let data = b"5";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 5);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_zero() {
        let data = b"0";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 0);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_no_digit_fails() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_minus_only_fails() {
        let data = b"-abc";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_i64_min() {
        let cursor = ByteCursor::new(b"-9223372036854775808");
        let (value, _) = i64().parse(cursor).unwrap();
        assert_eq!(value, i64::MIN);
    }

    #[test]
    fn test_below_i64_min_fails() {
        let cursor = ByteCursor::new(b"-9223372036854775809");
        assert!(i64().parse(cursor).is_err());
    }

    #[test]
    fn test_large_number() {
        let data = b"9876543210";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 9876543210);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
