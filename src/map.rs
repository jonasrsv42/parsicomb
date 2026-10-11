//! Transform a parser's output

use super::parser::Parser;

/// Parser combinator that transforms the output of a parser using a mapping function
pub struct Map<P, F> {
    parser: P,
    mapper: F,
}

impl<P, F> Map<P, F> {
    /// Same as [`map`]
    pub fn new(parser: P, mapper: F) -> Self {
        Map { parser, mapper }
    }
}

impl<P, F, Out> Parser for Map<P, F>
where
    P: Parser,
    F: Fn(P::Output) -> Out,
{
    type Cursor = P::Cursor;
    type Output = Out;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (value, cursor) = self.parser.parse(cursor)?;
        let mapped_value = (self.mapper)(value);
        Ok((mapped_value, cursor))
    }
}

/// Convenience function to create a Map parser
pub fn map<P, F, Out>(parser: P, mapper: F) -> Map<P, F>
where
    P: Parser,
    F: Fn(P::Output) -> Out,
{
    Map::new(parser, mapper)
}

/// Extension trait to add .map() method support for parsers
pub trait MapExt: Parser + Sized {
    /// Transform the output with `mapper`
    fn map<F, Out>(self, mapper: F) -> Map<Self, F>
    where
        F: Fn(Self::Output) -> Out,
    {
        Map::new(self, mapper)
    }
}

impl<P: Parser> MapExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::ascii::i64;
    use crate::byte::is_byte;
    use crate::or::OrExt;

    #[derive(Debug, PartialEq)]
    enum Token {
        Letter(char),
        Number(i64),
        Special(char),
    }

    #[test]
    fn test_map_byte_to_char() {
        let data = b"A";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').map(|byte| byte as char);

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, 'A');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_map_integer_to_string() {
        let data = b"123";
        let cursor = ByteCursor::new(data);
        let parser = i64().map(|num| format!("Number: {}", num));

        let (result, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result, "Number: 123");
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_map_to_enum() {
        let data = b"X";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'X').map(|byte| Token::Letter(byte as char));

        let (token, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(token, Token::Letter('X'));
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_map_chaining() {
        let data = b"5";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'5')
            .map(|byte| byte as char)
            .map(|ch| ch.to_digit(10).unwrap())
            .map(|digit| format!("Digit: {}", digit));

        let (result, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result, "Digit: 5");
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_map_with_or_common_enum() {
        let data = b"42";
        let cursor = ByteCursor::new(data);

        // Create parsers that map to a common enum type
        let letter_parser = is_byte(b'A').map(|byte| Token::Letter(byte as char));
        let number_parser = i64().map(Token::Number);
        let special_parser = is_byte(b'!').map(|byte| Token::Special(byte as char));

        // Now we can use or() since they all return Token
        let parser = letter_parser.or(number_parser).or(special_parser);

        let (token, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(token, Token::Number(42));
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_map_preserves_errors() {
        let data = b"xyz";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'A').map(|byte| byte as char);

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_function_syntax() {
        let data = b"9";
        let cursor = ByteCursor::new(data);
        let parser = map(is_byte(b'9'), |byte| byte as char);

        let (ch, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(ch, '9');
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
