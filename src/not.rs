use super::parser::Parser;
use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::error::{CodeLoc, ParsicombError};
use std::borrow::Cow;

/// Parser combinator that performs negative lookahead
///
/// Succeeds with () if the given parser fails at the current position.
/// Fails if the given parser succeeds.
/// Never consumes any input regardless of outcome.
pub struct Not<P> {
    parser: P,
}

impl<P> Not<P> {
    pub fn new(parser: P) -> Self {
        Not { parser }
    }
}

impl<'code, P> Parser<'code> for Not<P>
where
    P: Parser<'code>,
    P::Cursor: Cursor<'code>,
    <P::Cursor as Cursor<'code>>::Element: Atomic + 'code,
{
    type Cursor = P::Cursor;
    type Output = ();
    type Error = ParsicombError<'code, <P::Cursor as Cursor<'code>>::Element>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match self.parser.parse(cursor) {
            Ok(_) => {
                // Parser succeeded when we wanted it to fail
                let (data, position) = cursor.inner();
                Err(ParsicombError::SyntaxError {
                    message: Cow::Borrowed("negative lookahead failed: unexpected match"),
                    loc: CodeLoc::new(data, position),
                })
            }
            Err(_) => {
                // Parser failed as expected - return success without consuming input
                Ok(((), cursor))
            }
        }
    }
}

/// Convenience function to create a Not parser for negative lookahead
pub fn not<'code, P>(parser: P) -> Not<P>
where
    P: Parser<'code>,
{
    Not::new(parser)
}

/// Extension trait to add .not() method support for parsers
pub trait NotExt<'code>: Parser<'code> + Sized {
    fn not(self) -> Not<Self> {
        Not::new(self)
    }
}

/// Implement NotExt for all parsers
impl<'code, P> NotExt<'code> for P where P: Parser<'code> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::and::AndExt;
    use crate::byte::{byte, is_byte};
    use crate::many::many;
    use crate::map::MapExt;
    use crate::utf8::string::is_string;

    #[test]
    fn test_not_fails_on_match() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = not(is_string("hello"));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_not_succeeds_on_no_match() {
        let data = b"world";
        let cursor = ByteCursor::new(data);
        let parser = not(is_string("hello"));

        let ((), cursor) = parser.parse(cursor).unwrap();
        // Cursor should not have moved
        assert_eq!(cursor.value().unwrap(), b'w');
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn test_not_with_byte() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = not(is_byte(b'x'));

        let ((), cursor) = parser.parse(cursor).unwrap();
        // Cursor should not have moved
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_not_combined_with_byte() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        // Match any byte that's not 'x'
        let parser = not(is_byte(b'x')).and(byte());

        let (((), actual_byte), _cursor) = parser.parse(cursor).unwrap();
        assert_eq!(actual_byte, b'a');
    }

    #[test]
    fn test_not_for_parsing_until_delimiter() {
        let data = b"hello]]world";
        let cursor = ByteCursor::new(data);
        // Parse bytes until we see "]]"
        let parser = many(not(is_string("]]")).and(byte()).map(|(_, b)| b));

        let (bytes, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(bytes, vec![b'h', b'e', b'l', b'l', b'o']);

        // Verify we stopped at "]]"
        let next_two = is_string("]]").parse(remaining).unwrap();
        assert_eq!(next_two.0, "]]");
    }

    #[test]
    fn test_not_method_syntax() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        // Using .not() method
        let parser = is_string("hello").not();
        let ((), cursor) = parser.parse(cursor).unwrap();

        // Cursor should not have moved
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn test_not_comment_parsing_scenario() {
        let data = b"/* comment */ code";
        let cursor = ByteCursor::new(data);

        // Skip "/*"
        let cursor = is_string("/*").parse(cursor).unwrap().1;

        // Parse until "*/" but don't consume it
        let parser = many(not(is_string("*/")).and(byte()).map(|(_, b)| b));
        let (comment_bytes, cursor) = parser.parse(cursor).unwrap();

        let comment = String::from_utf8(comment_bytes).unwrap();
        assert_eq!(comment, " comment ");

        // Now we can parse the "*/"
        let (end_marker, _) = is_string("*/").parse(cursor).unwrap();
        assert_eq!(end_marker, "*/");
    }

    #[test]
    fn test_not_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = not(is_byte(b'a'));

        // Should succeed because there's no 'a' at position 0 (there's nothing)
        let ((), cursor) = parser.parse(cursor).unwrap();
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_not_at_end_of_file() {
        let data = b"a";
        let cursor = ByteCursor::new(data);
        let cursor = byte().parse(cursor).unwrap().1; // Move to EOF

        let parser = not(is_byte(b'a'));
        // Should succeed because we're at EOF, not at 'a'
        let result = parser.parse(cursor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_not_preserves_cursor_position() {
        let data = b"test string";
        let cursor = ByteCursor::new(data);

        // Move cursor forward a bit
        let cursor = is_string("test").parse(cursor).unwrap().1;
        assert_eq!(cursor.position(), 4);

        // Try not combinator
        let parser = not(is_string("xyz"));
        let ((), cursor_after) = parser.parse(cursor).unwrap();

        // Cursor should not have moved
        assert_eq!(cursor_after.position(), 4);
        assert_eq!(cursor_after.value().unwrap(), b' ');
    }
}
