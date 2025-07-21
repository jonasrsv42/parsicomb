use super::i64::i64;
use super::u64::u64;
use crate::and::AndExt;
use crate::byte::is_byte;
use crate::byte_cursor::ByteCursor;
use crate::parser::Parser;
use crate::{CodeLoc, ParsiCombError};

const MAX_FRACTIONAL_DIGITS: usize = 15;

/// Parser for int.uint format (e.g., 123.456, -42.789)
fn int_dot_uint<'code>() -> impl Parser<'code, Output = f64> {
    IntDotUintParser
}

struct IntDotUintParser;

impl<'code> Parser<'code> for IntDotUintParser {
    type Output = f64;

    fn parse(&self, cursor: ByteCursor<'code>) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>> {
        let (((int_part, _), frac_part), cursor) =
            i64().and(is_byte(b'.')).and(u64()).parse(cursor)?;

        let frac_digits = frac_part.to_string().len();

        // Check for too many fractional digits
        if frac_digits > MAX_FRACTIONAL_DIGITS {
            let (data, position) = cursor.inner();
            return Err(ParsiCombError::SyntaxError {
                message: format!(
                    "too many fractional digits: {} (max {})",
                    frac_digits, MAX_FRACTIONAL_DIGITS
                ),
                loc: CodeLoc::new(data, position)
            });
        }

        let frac_divisor = 10_f64.powi(frac_digits as i32);
        let fractional = frac_part as f64 / frac_divisor;

        // Check for integer part precision loss
        let int_as_f64 = int_part as f64;
        if int_as_f64 as i64 != int_part {
            let (data, position) = cursor.inner();
            return Err(ParsiCombError::SyntaxError {
                message: format!("integer part too large for f64 precision: {}", int_part),
                loc: CodeLoc::new(data, position)
            });
        }

        let result = if int_part >= 0 {
            int_as_f64 + fractional
        } else {
            int_as_f64 - fractional
        };

        // Check for overflow/infinity
        if !result.is_finite() {
            let (data, position) = cursor.inner();
            return Err(ParsiCombError::SyntaxError {
                message: "floating point overflow".to_string(),
                loc: CodeLoc::new(data, position)
            });
        }

        Ok((result, cursor))
    }
}

/// Parser that matches ASCII floating point numbers
pub fn f64<'code>() -> impl Parser<'code, Output = f64> {
    int_dot_uint()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_dot_uint() {
        let data = b"123.456abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - 123.456).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_negative_int_dot_uint() {
        let data = b"-42.789xyz";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - (-42.789)).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_dot_uint_fails() {
        let data = b".456abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_int_dot_fails() {
        let data = b"123.abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_int_dot_fails() {
        let data = b"-456.xyz";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_patterns() {
        let data = b"0.0";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();
        let (value, _) = parser.parse(cursor).unwrap();
        assert!((value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_match_fails() {
        let data = b"abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_just_dot_fails() {
        let data = b".abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_too_many_fractional_digits() {
        // 20 fractional digits (exceeds MAX_FRACTIONAL_DIGITS = 15)
        let data = b"1.12345678901234567890";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("too many fractional digits")
        );
    }

    #[test]
    fn test_max_fractional_digits_ok() {
        // Exactly 15 fractional digits should work
        let data = b"1.123456789012345";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = f64();

        let result = parser.parse(cursor);
        assert!(result.is_ok());
    }
}
