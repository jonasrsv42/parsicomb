//! Byte parsers: the [`atomic`](mod@crate::atomic) parsers for `u8` input

use crate::atomic::AtomicParser;
use crate::atomic::{AtomicBetween, IsAtomic, atomic_between, is_atomic};
use crate::{ByteCursor, atomic};

/// Parser that consumes and returns a single byte
pub type ByteParser<'code> = AtomicParser<ByteCursor<'code>>;

/// Parser that matches a specific byte
pub type IsByteParser<'code> = IsAtomic<ByteCursor<'code>>;

/// Parser that matches a byte within an inclusive range
pub type BetweenBytesParser<'code> = AtomicBetween<ByteCursor<'code>>;

/// Create a parser that consumes and returns a single byte
pub fn byte<'code>() -> ByteParser<'code> {
    atomic()
}

/// Create a parser that matches a specific byte
pub fn is_byte<'code>(expected: u8) -> IsByteParser<'code> {
    is_atomic(expected)
}

/// Create a parser that matches a byte within an inclusive range
pub fn between_bytes<'code>(start: u8, end: u8) -> BetweenBytesParser<'code> {
    atomic_between(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
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
                .contains("expected byte 0x68 ('h'), found byte 0x77 ('w')")
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
            msg.contains(r"expected byte 0xAA ('\xaa'), found byte 0xFF ('\xff')"),
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
                .contains("expected byte 0x30 ('0') to byte 0x39 ('9'), found byte 0x2F ('/')")
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
                .contains("expected byte 0x30 ('0') to byte 0x39 ('9'), found byte 0x3A (':')")
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
            msg.contains(r"expected byte 0x09 ('\t'), found byte 0x0A ('\n')"),
            "got {msg}"
        );
    }
}
