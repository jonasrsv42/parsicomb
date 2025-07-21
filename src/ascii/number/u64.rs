use crate::parser::Parser;
use crate::byte_cursor::ByteCursor;
use crate::some::some;
use crate::{ParsiCombError, CodeLoc};
use super::digit::digit;

/// Parser that matches one or more ASCII digits and returns them as a u64
pub fn u64<'code>() -> impl Parser<'code, Output = u64> {
    UIntParser
}

struct UIntParser;

impl<'code> Parser<'code> for UIntParser {
    type Output = u64;
    
    fn parse(&self, cursor: ByteCursor<'code>) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>> {
        let (digit_bytes, cursor) = some(digit()).parse(cursor)?;
        
        // Convert digits to string
        let num_str = match std::str::from_utf8(&digit_bytes) {
            Ok(s) => s,
            Err(_) => {
                let (data, position) = cursor.inner();
                return Err(ParsiCombError::SyntaxError {
                    message: "invalid UTF-8 in digits".to_string(),
                    loc: CodeLoc::new(data, position)
                });
            }
        };
        
        // Parse the number
        let value = match num_str.parse::<u64>() {
            Ok(v) => v,
            Err(_) => {
                let (data, position) = cursor.inner();
                return Err(ParsiCombError::SyntaxError {
                    message: format!("number too large: {}", num_str),
                    loc: CodeLoc::new(data, position)
                });
            }
        };
        
        Ok((value, cursor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_cursor::ByteCursor;

    #[test]
    fn test_uint_single_digit() {
        let data = b"5abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = u64();
        
        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 5);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_uint_multiple_digits() {
        let data = b"123abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = u64();
        
        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 123);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_uint_zero() {
        let data = b"0";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = u64();
        
        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 0);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_uint_large_number() {
        let data = b"9876543210";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = u64();
        
        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 9876543210);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_uint_no_digit_fails() {
        let data = b"abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = u64();
        
        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_uint_stops_at_non_digit() {
        let data = b"42.5";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = u64();
        
        let (value, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(value, 42);
        assert_eq!(cursor.value().unwrap(), b'.');
    }

    #[test]
    fn test_uint_overflow() {
        // This number is larger than u64::MAX
        let data = b"99999999999999999999999999999999";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = u64();
        
        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("number too large"));
    }
}
