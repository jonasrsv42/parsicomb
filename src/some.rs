use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use crate::ParsiCombError;

/// Parser combinator that matches one or more occurrences of the given parser
pub struct Some<P> {
    parser: P,
}

impl<P> Some<P> {
    pub fn new(parser: P) -> Self {
        Some { parser }
    }
}

impl<'code, P> Parser<'code> for Some<P>
where
    P: Parser<'code>,
{
    type Output = Vec<P::Output>;
    
    fn parse(&self, cursor: ByteCursor<'code>) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>> {
        let mut results = Vec::new();
        
        // First parse must succeed
        let (first_value, mut cursor) = self.parser.parse(cursor)?;
        results.push(first_value);
        
        // Continue parsing zero or more times
        loop {
            match self.parser.parse(cursor) {
                Ok((value, next_cursor)) => {
                    results.push(value);
                    cursor = next_cursor;
                }
                Err(_) => {
                    // Stop on first error after at least one match
                    break;
                }
            }
        }
        
        Ok((results, cursor))
    }
}

/// Convenience function to create a Some parser
pub fn some<'code, P>(parser: P) -> Some<P>
where
    P: Parser<'code>,
{
    Some::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte::{ByteParser, is_byte};

    #[test]
    fn test_some_zero_matches_fails() {
        let data = b"xyz";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = some(is_byte(b'a'));
        
        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_some_one_match() {
        let data = b"abc";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = some(is_byte(b'a'));
        
        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_some_multiple_matches() {
        let data = b"aaabcd";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = some(is_byte(b'a'));
        
        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_some_all_matches() {
        let data = b"aaaa";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = some(is_byte(b'a'));
        
        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a', b'a']);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_some_with_byte_parser() {
        let data = b"hello";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = some(ByteParser::new());
        
        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'h', b'e', b'l', b'l', b'o']);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_some_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = some(is_byte(b'a'));
        
        let result = parser.parse(cursor);
        assert!(result.is_err());
    }
}
