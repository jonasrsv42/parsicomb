use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use crate::{CodeLoc, ParsiCombError};

/// Parser that consumes and returns a single byte
pub struct ByteParser;

impl ByteParser {
    pub fn new() -> Self {
        ByteParser
    }
}

/// Convenience function to create a ByteParser
pub fn byte() -> ByteParser {
    ByteParser::new()
}

impl<'code> Parser<'code> for ByteParser {
    type Output = u8;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>> {
        let byte = cursor.value()?;
        Ok((byte, cursor.next()))
    }
}

/// Parser that matches a specific byte
pub struct IsByteParser {
    expected: u8,
}

impl IsByteParser {
    pub fn new(expected: u8) -> Self {
        IsByteParser { expected }
    }
}

impl<'code> Parser<'code> for IsByteParser {
    type Output = u8;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>> {
        match cursor.value() {
            Ok(byte) if byte == self.expected => Ok((byte, cursor.next())),
            Ok(byte) => {
                let (data, position) = cursor.inner();
                let message = format!(
                    "expected byte 0x{:02X} ('{}'), found 0x{:02X} ('{}')",
                    self.expected,
                    std::str::from_utf8(&[self.expected]).unwrap_or("<non-utf8>"),
                    byte,
                    std::str::from_utf8(&[byte]).unwrap_or("<non-utf8>")
                );
                Err(ParsiCombError::SyntaxError {
                    message,
                    loc: CodeLoc::new(data, position),
                })
            }
            Err(e) => Err(e),
        }
    }
}

/// Parser that matches a byte within a range (inclusive)
pub struct BetweenBytesParser {
    start: u8,
    end: u8,
}

impl BetweenBytesParser {
    pub fn new(start: u8, end: u8) -> Self {
        BetweenBytesParser { start, end }
    }
}

impl<'code> Parser<'code> for BetweenBytesParser {
    type Output = u8;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>> {
        match cursor.value() {
            Ok(byte) if byte >= self.start && byte <= self.end => Ok((byte, cursor.next())),
            Ok(byte) => {
                let (data, position) = cursor.inner();
                let message = format!(
                    "expected byte in range 0x{:02X}-0x{:02X} ('{}'-'{}'), found 0x{:02X} ('{}')",
                    self.start,
                    self.end,
                    std::str::from_utf8(&[self.start]).unwrap_or("<non-utf8>"),
                    std::str::from_utf8(&[self.end]).unwrap_or("<non-utf8>"),
                    byte,
                    std::str::from_utf8(&[byte]).unwrap_or("<non-utf8>")
                );
                Err(ParsiCombError::SyntaxError {
                    message,
                    loc: CodeLoc::new(data, position)
                })
            }
            Err(e) => Err(e),
        }
    }
}

/// Convenience function to create an IsByteParser
pub fn is_byte(expected: u8) -> IsByteParser {
    IsByteParser::new(expected)
}

/// Convenience function to create a BetweenBytesParser
pub fn between_bytes(start: u8, end: u8) -> BetweenBytesParser {
    BetweenBytesParser::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_parser_success() {
        let data = b"hello";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = ByteParser::new();

        let result = parser.parse(cursor).unwrap();
        let (byte, next_cursor) = result;

        assert_eq!(byte, b'h');
        assert_eq!(next_cursor.value().unwrap(), b'e');
    }

    #[test]
    fn test_byte_parser_eof() {
        let data = b"x";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = ByteParser::new();

        // First parse succeeds
        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'x');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));

        // Second parse fails with EOF
        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_byte_parser_sequence() {
        let data = b"abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = ByteParser::new();

        let (b1, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(b1, b'a');

        let (b2, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(b2, b'b');

        let (b3, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(b3, b'c');

        // Next byte should be EOF
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_is_byte_parser_success() {
        let data = b"hello";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = is_byte(b'h');

        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'h');
        assert_eq!(cursor.value().unwrap(), b'e');
    }

    #[test]
    fn test_is_byte_parser_failure() {
        let data = b"world";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = is_byte(b'h');

        let result = parser.parse(cursor);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("expected byte 0x68 ('h'), found 0x77 ('w')")
        );
    }

    #[test]
    fn test_is_byte_parser_non_utf8() {
        let data = &[0xFF, 0xFE];
        let cursor = ByteCursor::new(data).unwrap();
        let parser = is_byte(0xAA);

        let result = parser.parse(cursor);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("<non-utf8>"));
    }

    #[test]
    fn test_in_range_parser_success() {
        let data = b"5abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between_bytes(b'0', b'9');

        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'5');
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_in_range_parser_failure_below() {
        let data = b"/abc"; // '/' is 0x2F, '0' is 0x30
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between_bytes(b'0', b'9');

        let result = parser.parse(cursor);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("expected byte in range 0x30-0x39 ('0'-'9'), found 0x2F ('/')")
        );
    }

    #[test]
    fn test_in_range_parser_failure_above() {
        let data = b":abc"; // ':' is 0x3A, '9' is 0x39
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between_bytes(b'0', b'9');

        let result = parser.parse(cursor);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("expected byte in range 0x30-0x39 ('0'-'9'), found 0x3A (':')")
        );
    }

    #[test]
    fn test_in_range_parser_eof() {
        let data = b"";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = between_bytes(b'a', b'z');

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }
}
