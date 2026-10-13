//! Zero or more repetitions, collected into a `Vec`

use super::parser::Parser;
use crate::cursor::Cursor;
use crate::error::{Advance, ErrorNode};

/// Parser combinator that matches zero or more occurrences of the given parser
///
/// Stops when the parser fails without advancing ([`Advance::None`]); any
/// other failure is passed on (see [`Advance`]).
pub struct Many<P> {
    parser: P,
}

impl<P> Many<P> {
    /// Same as [`many`]
    pub fn new(parser: P) -> Self {
        Many { parser }
    }
}

impl<P> Parser for Many<P>
where
    P: Parser,
{
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = P::Error;

    fn parse(&self, mut cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::new();

        loop {
            match self.parser.parse(cursor) {
                // An iteration that consumes nothing would repeat forever
                Ok((_, next_cursor)) if next_cursor.position() == cursor.position() => break,
                Ok((value, next_cursor)) => {
                    results.push(value);
                    cursor = next_cursor;
                }
                Err(error) => match error.progress().since(cursor.position()) {
                    Advance::None => break,
                    Advance::Partial | Advance::Committed => return Err(error),
                },
            }
        }

        Ok((results, cursor))
    }
}

/// Convenience function to create a Many parser
pub fn many<P>(parser: P) -> Many<P>
where
    P: Parser,
{
    Many::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::byte::{ByteParser, is_byte};

    #[test]
    fn test_many_zero_matches() {
        let data = b"xyz";
        let cursor = ByteCursor::new(data);
        let parser = many(is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![]);
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_many_one_match() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = many(is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_many_multiple_matches() {
        let data = b"aaabcd";
        let cursor = ByteCursor::new(data);
        let parser = many(is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_many_all_matches() {
        let data = b"aaaa";
        let cursor = ByteCursor::new(data);
        let parser = many(is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a', b'a']);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_many_with_byte_parser() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = many(ByteParser::new());

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'h', b'e', b'l', b'l', b'o']);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_many_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = many(is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![]);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_many_non_consuming_parser_terminates() {
        let cursor = ByteCursor::new(b"b");
        let parser = many(crate::optional(is_byte(b'a')));

        let (output, remaining) = parser.parse(cursor).unwrap();
        assert!(output.is_empty());
        assert_eq!(remaining.value().unwrap(), b'b');
    }

    #[test]
    fn test_many_of_many_terminates() {
        let cursor = ByteCursor::new(b"aab");
        let parser = many(many(is_byte(b'a')));

        let (output, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(output, vec![vec![b'a', b'a']]);
        assert_eq!(remaining.value().unwrap(), b'b');
    }
}
