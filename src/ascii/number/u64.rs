use super::digit::digit;
use crate::ByteCursor;
use crate::ParsicombError;
use crate::cursor::Cursor;
use crate::parser::Parser;
use crate::position::PositionExt;
use crate::some::some;
use std::marker::PhantomData;

/// Parser that matches one or more ASCII digits and returns them as a u64
pub fn u64<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = u64, Error = ParsicombError<'code>> {
    UIntParser {
        _input: PhantomData,
    }
}

struct UIntParser<'code> {
    _input: PhantomData<&'code [u8]>,
}

impl<'code> Parser for UIntParser<'code> {
    type Cursor = ByteCursor<'code>;
    type Output = u64;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let ((_, digits), end) = some(digit()).with_position().parse(cursor)?;

        // ASCII digits only, so both conversions fail only on overflow
        let value = std::str::from_utf8(digits.slice())
            .ok()
            .and_then(|text| text.parse::<u64>().ok())
            .ok_or_else(|| {
                end.syntax_error(format!("number too large: {}", digits.as_string()).into())
            })?;

        Ok((value, end))
    }
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
