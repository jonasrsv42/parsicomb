use super::u64::u64;
use crate::ByteCursor;
use crate::Cursor;
use crate::parser::Parser;
use crate::{CodeLoc, ParsicombError};

/// Parser that matches ASCII integer numbers (positive or negative)
pub fn i64<'code>()
-> impl Parser<'code, Cursor = ByteCursor<'code>, Output = i64, Error = ParsicombError<'code>> {
    IntParser
}

struct IntParser;

impl<'code> Parser<'code> for IntParser {
    type Cursor = ByteCursor<'code>;
    type Output = i64;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut cursor = cursor;
        let mut is_negative = false;

        // Check for optional sign
        match cursor.value() {
            Ok(b'-') => {
                is_negative = true;
                cursor = cursor.next();
            }
            Ok(b'+') => {
                // Skip optional plus sign
                cursor = cursor.next();
            }
            _ => {}
        }

        // Parse the unsigned integer part
        let (value, cursor) = u64().parse(cursor)?;

        // Convert to signed and apply sign
        let signed_value = if is_negative {
            // Check for overflow when negating
            if value > i64::MAX as u64 + 1 {
                let (data, position) = cursor.inner();
                return Err(ParsicombError::SyntaxError {
                    message: format!("negative number too large: -{}", value).into(),
                    loc: CodeLoc::new(data, position),
                });
            }
            -(value as i64)
        } else {
            // Check for positive overflow
            if value > i64::MAX as u64 {
                let (data, position) = cursor.inner();
                return Err(ParsicombError::SyntaxError {
                    message: format!("positive number too large: {}", value).into(),
                    loc: CodeLoc::new(data, position),
                });
            }
            value as i64
        };

        Ok((signed_value, cursor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_large_number() {
        let data = b"9876543210";
        let cursor = ByteCursor::new(data);
        let parser = i64();

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 9876543210);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
