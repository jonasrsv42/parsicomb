//! Repeat a parser until its output satisfies a predicate

use crate::Cursor;
use crate::parser::Parser;

/// Parser that repeatedly applies another parser until a predicate is satisfied
///
/// Collects the outputs before the first one that satisfies `predicate`, and
/// stops *before* that item, leaving it unconsumed. It also stops, successfully,
/// at the end of input (whether or not a terminator was found) and when an
/// item consumes nothing. Unlike [`many`](crate::many()), a failure of
/// `parser` fails the whole `take_until`.
pub struct TakeUntil<P, F> {
    parser: P,
    predicate: F,
}

impl<P, F> TakeUntil<P, F> {
    /// Same as [`take_until`]
    pub fn new(parser: P, predicate: F) -> Self {
        Self { parser, predicate }
    }
}

impl<P, F> Parser for TakeUntil<P, F>
where
    P: Parser,
    F: Fn(&P::Output) -> bool,
{
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut result = Vec::new();
        let mut current_cursor = cursor;

        loop {
            // Check if we've reached end of input
            if current_cursor.eos() {
                return Ok((result, current_cursor));
            }

            // Try to parse the next item
            match self.parser.parse(current_cursor) {
                Ok((item, new_cursor)) => {
                    // Check if predicate is satisfied (stop condition)
                    if (self.predicate)(&item) {
                        // Don't consume the item that satisfied the predicate
                        return Ok((result, current_cursor));
                    } else if new_cursor.position() == current_cursor.position() {
                        // An iteration that consumes nothing would repeat forever
                        return Ok((result, current_cursor));
                    } else {
                        // Add item to result and continue
                        result.push(item);
                        current_cursor = new_cursor;
                    }
                }
                Err(error) => {
                    // Parser failed - propagate the error
                    return Err(error);
                }
            }
        }
    }
}

/// Create a parser that collects outputs of `parser` until one satisfies
/// `predicate`; see [`TakeUntil`]
pub fn take_until<P, F>(parser: P, predicate: F) -> TakeUntil<P, F>
where
    P: Parser,
    F: Fn(&P::Output) -> bool,
{
    TakeUntil::new(parser, predicate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::byte::byte;
    use crate::utf8::char::char;

    #[test]
    fn test_take_until_char_quote() {
        let input = r#"hello world"more"#;
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| *c == '"');

        let (result, _) = parser.parse(cursor).unwrap();
        let result_string: String = result.into_iter().collect();
        assert_eq!(result_string, "hello world");
    }

    #[test]
    fn test_take_until_char_backslash() {
        let input = r#"hello\world"#;
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| *c == '\\');

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        let result_string: String = result.into_iter().collect();
        assert_eq!(result_string, "hello");

        // Should be positioned at the backslash
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, '\\');
    }

    #[test]
    fn test_take_until_byte_newline() {
        let input = b"hello\nworld";
        let cursor = ByteCursor::new(input);
        let parser = take_until(byte(), |b: &u8| *b == b'\n');

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result, vec![b'h', b'e', b'l', b'l', b'o']);

        // Should be positioned at the newline
        let (next_byte, _) = byte().parse(remaining_cursor).unwrap();
        assert_eq!(next_byte, b'\n');
    }

    #[test]
    fn test_take_until_unicode() {
        let input = "température🦀world";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| *c == '🦀');

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        let result_string: String = result.into_iter().collect();
        assert_eq!(result_string, "température");

        // Should be positioned at the crab emoji
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, '🦀');
    }

    #[test]
    fn test_take_until_multiple_conditions() {
        let input = "hello,world";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| *c == ',' || *c == ';');

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        let result_string: String = result.into_iter().collect();
        assert_eq!(result_string, "hello");

        // Should be positioned at the comma
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, ',');
    }

    #[test]
    fn test_take_until_not_found() {
        let input = "hello world";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| *c == 'x');

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        let result_string: String = result.into_iter().collect();
        assert_eq!(result_string, "hello world");

        // Should be at end of input
        assert!(matches!(remaining_cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_take_until_empty_result() {
        let input = "\"hello";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| *c == '"');

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result.len(), 0);

        // Should be positioned at the quote
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, '"');
    }

    #[test]
    fn test_take_until_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| *c == '"');

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result.len(), 0);
        assert!(matches!(remaining_cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_take_until_string_parsing_scenario() {
        // Simulate parsing string content until escape or quote
        let input = r#"Hello, world!\nNext"#;
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| *c == '"' || *c == '\\');

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        let result_string: String = result.into_iter().collect();
        assert_eq!(result_string, "Hello, world!");

        // Should be positioned at the backslash
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, '\\');
    }

    #[test]
    fn test_take_until_predicate_with_context() {
        // Test using a more complex predicate
        let input = "abc123def";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = take_until(char(), |c: &char| c.is_numeric());

        let (result, remaining_cursor) = parser.parse(cursor).unwrap();
        let result_string: String = result.into_iter().collect();
        assert_eq!(result_string, "abc");

        // Should be positioned at the first digit
        let (next_char, _) = char().parse(remaining_cursor).unwrap();
        assert_eq!(next_char, '1');
    }

    #[test]
    fn test_take_until_stops_when_parser_consumes_nothing() {
        use crate::byte::is_byte;
        use crate::optional::optional;

        let data = b"b";
        let cursor = ByteCursor::new(data);
        let parser = take_until(optional(is_byte(b'a')), |o| *o == Some(b'z'));

        let (result, cursor) = parser.parse(cursor).unwrap();
        assert!(result.is_empty());
        assert_eq!(cursor.value().unwrap(), b'b');
    }
}
