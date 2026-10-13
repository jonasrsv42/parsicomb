//! Zero or one occurrence

use super::parser::Parser;
use crate::cursor::Cursor;
use crate::error::{Advance, ErrorNode};

/// Parser combinator that optionally matches the given parser
///
/// Returns `Some(value)` if the parser succeeds, and `None` without moving
/// the cursor if it fails without advancing ([`Advance::None`]); any other
/// failure is passed on (see [`Advance`]).
///
/// So `optional(p)` is not the same as `p.map(Some).or(default(None))`: `or`
/// also backtracks from a `p` that matched part of its input and then failed,
/// and drops that error.
pub struct Optional<P> {
    parser: P,
}

impl<P> Optional<P> {
    /// Same as [`optional`]
    pub fn new(parser: P) -> Self {
        Optional { parser }
    }
}

impl<P> Parser for Optional<P>
where
    P: Parser,
{
    type Cursor = P::Cursor;
    type Output = Option<P::Output>;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match self.parser.parse(cursor) {
            Ok((value, next_cursor)) => Ok((Some(value), next_cursor)),
            Err(error) => match error.progress().since(cursor.position()) {
                Advance::None => Ok((None, cursor)),
                Advance::Partial | Advance::Committed => Err(error),
            },
        }
    }
}

/// Convenience function to create an Optional parser
pub fn optional<P>(parser: P) -> Optional<P>
where
    P: Parser,
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
