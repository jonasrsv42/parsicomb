//! ASCII floats: `int.frac`, with an optional exponent, or `int` and an
//! exponent

use super::{digits, exponent_end, float_value, sign};
use crate::ByteCursor;
use crate::ParsicombError;
use crate::byte::is_byte;
use crate::optional::optional;
use crate::parser::Parser;
use crate::parser_fn::parser_fn;

/// Parser that matches ASCII floating point numbers in `int.uint` form with
/// an optional exponent, or an integer with an exponent (e.g. `123.456`,
/// `-42.789`, `+0.5`, `1.5e3`, `2E-8`, `1e5`)
///
/// An `e` not followed by an exponent's digits is left unmatched: `1.5e`
/// parses as `1.5`.
///
/// Fails after the number if it overflows an f64; a number too
/// small for an f64 becomes 0.0, as with [`str::parse`].
pub fn f64<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = f64, Error = ParsicombError<'code>> {
    parser_fn(|start: ByteCursor<'code>| {
        // Match [+-]digits(.digits[exponent] | exponent)
        let (_, end) = optional(sign()).parse(start)?;
        let (_, end) = digits().parse(end)?;
        let end = match exponent_end(end) {
            Some(end) => end,
            None => {
                let (_, end) = is_byte(b'.').parse(end)?;
                let (_, end) = digits().parse(end)?;
                exponent_end(end).unwrap_or(end)
            }
        };
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

    #[test]
    fn test_exponents() {
        let parse = |input: &[u8]| {
            let (value, cursor) = f64().parse(ByteCursor::new(input)).unwrap();
            (value, cursor.position())
        };
        assert_eq!(parse(b"1.5e3"), (1500.0, 5));
        assert_eq!(parse(b"1.5E-1x"), (0.15, 6));
        assert_eq!(parse(b"1e5"), (100000.0, 3));
        assert_eq!(parse(b"-2e+1"), (-20.0, 5));
        assert_eq!(parse(b"1.5e"), (1.5, 3));
        assert_eq!(parse(b"1.5e+x"), (1.5, 3));
        // An integer needs a fraction or an exponent
        assert!(f64().parse(ByteCursor::new(b"1ex")).is_err());
    }
}
