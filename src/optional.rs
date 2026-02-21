use super::parser::Parser;

/// Parser combinator that optionally matches the given parser.
/// Returns Some(value) if the parser succeeds, None if it fails.
/// Never fails - on parser failure, returns None and leaves cursor unchanged.
pub struct Optional<P> {
    parser: P,
}

impl<P> Optional<P> {
    pub fn new(parser: P) -> Self {
        Optional { parser }
    }
}

impl<'code, P> Parser<'code> for Optional<P>
where
    P: Parser<'code>,
{
    type Cursor = P::Cursor;
    type Output = Option<P::Output>;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match self.parser.parse(cursor) {
            Ok((value, next_cursor)) => Ok((Some(value), next_cursor)),
            Err(_) => Ok((None, cursor)),
        }
    }
}

/// Convenience function to create an Optional parser
pub fn optional<'code, P>(parser: P) -> Optional<P>
where
    P: Parser<'code>,
{
    Optional::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::byte::is_byte;

    #[test]
    fn test_optional_match() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = optional(is_byte(b'a'));

        let (result, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result, Some(b'a'));
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_optional_no_match() {
        let data = b"xyz";
        let cursor = ByteCursor::new(data);
        let parser = optional(is_byte(b'a'));

        let (result, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result, None);
        assert_eq!(cursor.value().unwrap(), b'x'); // cursor unchanged
    }

    #[test]
    fn test_optional_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = optional(is_byte(b'a'));

        let (result, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(result, None);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }
}
