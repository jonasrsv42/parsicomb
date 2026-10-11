use crate::cursor::Cursor;
use crate::error::Expected;
use crate::{AtomicParser, ByteCursor, Parser, ParsicombError, atomic};
use std::marker::PhantomData;

/// Type alias for a parser that consumes and returns a single byte
pub type ByteParser<'code> = AtomicParser<ByteCursor<'code>>;

/// Convenience function to create a ByteParser
pub fn byte<'code>() -> ByteParser<'code> {
    atomic()
}

/// Parser that matches a specific byte
pub struct IsByteParser<'code> {
    expected: u8,
    _input: PhantomData<&'code [u8]>,
}

impl<'code> IsByteParser<'code> {
    pub fn new(expected: u8) -> Self {
        IsByteParser {
            expected,
            _input: PhantomData,
        }
    }
}

impl<'code> Parser for IsByteParser<'code> {
    type Cursor = ByteCursor<'code>;
    type Output = u8;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match cursor.value()? {
            byte if byte == self.expected => Ok((byte, cursor.next())),
            found => Err(ParsicombError::Expected {
                expected: Expected::Byte {
                    expected: self.expected,
                    found,
                },
                loc: cursor.loc(),
            }),
        }
    }
}

/// Parser that matches a byte within a range (inclusive)
pub struct BetweenBytesParser<'code> {
    start: u8,
    end: u8,
    _input: PhantomData<&'code [u8]>,
}

impl<'code> BetweenBytesParser<'code> {
    pub fn new(start: u8, end: u8) -> Self {
        BetweenBytesParser {
            start,
            end,
            _input: PhantomData,
        }
    }
}

impl<'code> Parser for BetweenBytesParser<'code> {
    type Cursor = ByteCursor<'code>;
    type Output = u8;
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match cursor.value()? {
            byte if (self.start..=self.end).contains(&byte) => Ok((byte, cursor.next())),
            found => Err(ParsicombError::Expected {
                expected: Expected::ByteInRange {
                    start: self.start,
                    end: self.end,
                    found,
                },
                loc: cursor.loc(),
            }),
        }
    }
}

/// Convenience function to create an IsByteParser
pub fn is_byte<'code>(expected: u8) -> IsByteParser<'code> {
    IsByteParser::new(expected)
}

/// Convenience function to create a BetweenBytesParser
pub fn between_bytes<'code>(start: u8, end: u8) -> BetweenBytesParser<'code> {
    BetweenBytesParser::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::Cursor;
    use crate::cursors::atomic::AtomicCursor;

    #[test]
    fn test_byte_parser_success() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = byte();

        let result = parser.parse(cursor).unwrap();
        let (byte, next_cursor) = result;

        assert_eq!(byte, b'h');
        assert_eq!(next_cursor.value().unwrap(), b'e');
    }

    #[test]
    fn test_byte_parser_eof() {
        let data = b"x";
        let cursor = ByteCursor::new(data);
        let parser = byte();

        // First parse succeeds
        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'x');
        assert!(matches!(cursor, AtomicCursor::EndOfFile { .. }));

        // Second parse fails with EOF
        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_byte_parser_sequence() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = byte();

        let (b1, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(b1, b'a');

        let (b2, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(b2, b'b');

        let (b3, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(b3, b'c');

        // Next byte should be EOF
        assert!(matches!(cursor, AtomicCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_is_byte_parser_success() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'h');

        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'h');
        assert_eq!(cursor.value().unwrap(), b'e');
    }

    #[test]
    fn test_is_byte_parser_failure() {
        let data = b"world";
        let cursor = ByteCursor::new(data);
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
        let cursor = ByteCursor::new(data);
        let parser = is_byte(0xAA);

        let result = parser.parse(cursor);
        assert!(result.is_err());
        let error = result.unwrap_err();
        let msg = error.to_string();
        assert!(
            msg.contains(r"expected byte 0xAA ('\xaa'), found 0xFF ('\xff')"),
            "got {msg}"
        );
    }

    #[test]
    fn test_in_range_parser_success() {
        let data = b"5abc";
        let cursor = ByteCursor::new(data);
        let parser = between_bytes(b'0', b'9');

        let (byte, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'5');
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_in_range_parser_failure_below() {
        let data = b"/abc"; // '/' is 0x2F, '0' is 0x30
        let cursor = ByteCursor::new(data);
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
        let cursor = ByteCursor::new(data);
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
        let cursor = ByteCursor::new(data);
        let parser = between_bytes(b'a', b'z');

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_control_bytes_are_escaped_in_message() {
        let data = b"\n";
        let err = is_byte(b'\t').parse(ByteCursor::new(data)).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains(r"expected byte 0x09 ('\t'), found 0x0A ('\n')"),
            "got {msg}"
        );
    }
}
