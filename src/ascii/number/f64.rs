//! ASCII floats in `int.frac` form

use super::{digits, float_value, sign};
use crate::ByteCursor;
use crate::ParsicombError;
use crate::byte::is_byte;
use crate::optional::optional;
use crate::parser::Parser;
use crate::parser_fn::parser_fn;

/// Parser that matches ASCII floating point numbers in `int.uint` form
/// (e.g. `123.456`, `-42.789`, `+0.5`)
///
/// Fails at the start of the number if it overflows an f64; a number too
/// small for an f64 becomes 0.0, as with [`str::parse`].
pub fn f64<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = f64, Error = ParsicombError<'code>> {
    parser_fn(|start: ByteCursor<'code>| {
        // Match [+-]digits.digits
        let (_, end) = optional(sign()).parse(start)?;
        let (_, end) = digits().parse(end)?;
        let (_, end) = is_byte(b'.').parse(end)?;
        let (_, end) = digits().parse(end)?;
        Ok((float_value(start, end)?, end))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cursor;

    #[test]
    fn test_int_dot_uint() {
        let data = b"123.456abc";
        let cursor = ByteCursor::new(data);
        let parser = f64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - 123.456).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_negative_int_dot_uint() {
        let data = b"-42.789xyz";
        let cursor = ByteCursor::new(data);
        let parser = f64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - (-42.789)).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_dot_uint_fails() {
        let data = b".456abc";
        let cursor = ByteCursor::new(data);
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_int_dot_fails() {
        let data = b"123.abc";
        let cursor = ByteCursor::new(data);
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_int_dot_fails() {
        let data = b"-456.xyz";
        let cursor = ByteCursor::new(data);
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_patterns() {
        let data = b"0.0";
        let cursor = ByteCursor::new(data);
        let parser = f64();
        let (value, _) = parser.parse(cursor).unwrap();
        assert!((value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_match_fails() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_just_dot_fails() {
        let data = b".abc";
        let cursor = ByteCursor::new(data);
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    fn parse(data: &[u8]) -> f64 {
        f64().parse(ByteCursor::new(data)).unwrap().0
    }

    #[test]
    fn test_leading_zeros_in_fraction() {
        assert_eq!(parse(b"1.05"), 1.05);
        assert_eq!(parse(b"3.007"), 3.007);
        assert_eq!(parse(b"0.000001"), 0.000001);
    }

    #[test]
    fn test_negative_zero_integer_part() {
        assert_eq!(parse(b"-0.5"), -0.5);
        assert!(parse(b"-0.0").is_sign_negative());
    }

    #[test]
    fn test_plus_sign() {
        assert_eq!(parse(b"+1.5"), 1.5);
    }

    #[test]
    #[allow(clippy::excessive_precision)] // the point: more digits than f64 holds
    fn test_many_fractional_digits() {
        assert_eq!(parse(b"1.12345678901234567890"), 1.12345678901234567890);
    }

    #[test]
    fn test_large_integer_part() {
        assert_eq!(parse(b"9007199254740993.0"), 9007199254740993.0);
        assert_eq!(parse(b"100000000000000000000.5"), 1e20);
    }

    #[test]
    fn test_overflow_fails() {
        let mut data = vec![b'9'; 400];
        data.extend_from_slice(b".0");
        let result = f64().parse(ByteCursor::new(&data));
        assert!(result.unwrap_err().to_string().contains("overflow"));
    }

    #[test]
    fn test_sign_only_fails() {
        assert!(f64().parse(ByteCursor::new(b"-.5")).is_err());
        assert!(f64().parse(ByteCursor::new(b"-")).is_err());
    }
}
