//! One or more repetitions, collected into a `Vec`

use super::parser::Parser;
use crate::cursor::Cursor;
use crate::error::{Advance, ErrorNode};

/// Parser combinator that matches one or more occurrences of the given parser
///
/// A glob import of this module (`use parsicomb::some::*`) shadows
/// [`Option::Some`]; import [`some`] by name instead.
///
/// Stops when the parser fails without advancing ([`Advance::None`]); any
/// other failure is passed on (see [`Advance`]).
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Some<P> {
    parser: P,
}

impl<P> Some<P> {
    /// Same as [`some`]
    pub fn new(parser: P) -> Self {
        Some { parser }
    }
}

impl<P> Parser for Some<P>
where
    P: Parser,
{
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::new();

        // First parse must succeed
        let (first_value, mut cursor) = self.parser.parse(cursor)?;
        results.push(first_value);

        // Continue parsing zero or more times
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

/// Create a parser for one or more repetitions of `parser`
pub fn some<P>(parser: P) -> Some<P>
where
    P: Parser,
{
    Some::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::byte::{ByteParser, is_byte};

    #[test]
    fn test_some_zero_matches_fails() {
        let data = b"xyz";
        let cursor = ByteCursor::new(data);
        let parser = some(is_byte(b'a'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_some_one_match() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = some(is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_some_multiple_matches() {
        let data = b"aaabcd";
        let cursor = ByteCursor::new(data);
        let parser = some(is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_some_all_matches() {
        let data = b"aaaa";
        let cursor = ByteCursor::new(data);
        let parser = some(is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a', b'a']);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_some_with_byte_parser() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = some(ByteParser::new());

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'h', b'e', b'l', b'l', b'o']);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_some_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = some(is_byte(b'a'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_some_non_consuming_parser_terminates() {
        let cursor = ByteCursor::new(b"b");
        let parser = some(crate::optional(is_byte(b'a')));

        // The first (empty) match is required and kept; repetition then stops
        let (output, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(output, vec![None]);
        assert_eq!(remaining.value().unwrap(), b'b');
    }
}
