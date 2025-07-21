use crate::byte::is_byte;
use crate::or::OrExt;
use crate::parser::Parser;

/// Parser that matches a single ASCII whitespace character (space, tab, newline, carriage return)
pub fn whitespace<'a>() -> impl Parser<'a, Output = u8> {
    is_byte(b' ')
        .or(is_byte(b'\t'))
        .or(is_byte(b'\n'))
        .or(is_byte(b'\r'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_cursor::ByteCursor;
    use crate::many::many;

    #[test]
    fn test_whitespace_parser_space() {
        let data = b" abc";
        let cursor = ByteCursor::new(data);
        let parser = whitespace();

        let (ws, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ws, b' ');
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_whitespace_parser_tab() {
        let data = b"\txyz";
        let cursor = ByteCursor::new(data);
        let parser = whitespace();

        let (ws, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ws, b'\t');
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_whitespace_parser_newline() {
        let data = b"\nabc";
        let cursor = ByteCursor::new(data);
        let parser = whitespace();

        let (ws, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ws, b'\n');
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_whitespace_parser_carriage_return() {
        let data = b"\rxyz";
        let cursor = ByteCursor::new(data);
        let parser = whitespace();

        let (ws, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ws, b'\r');
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_whitespace_parser_non_whitespace_fails() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = whitespace();

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected byte"));
    }

    #[test]
    fn test_whitespaces_parser_zero_matches() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = many(whitespace());

        let (ws_vec, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ws_vec, vec![]);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_whitespaces_parser_multiple_matches() {
        let data = b"  \t\n abc";
        let cursor = ByteCursor::new(data);
        let parser = many(whitespace());

        let (ws_vec, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ws_vec, vec![b' ', b' ', b'\t', b'\n', b' ']);
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_whitespaces_parser_all_whitespace() {
        let data = b" \t\n\r";
        let cursor = ByteCursor::new(data);
        let parser = many(whitespace());

        let (ws_vec, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ws_vec, vec![b' ', b'\t', b'\n', b'\r']);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
