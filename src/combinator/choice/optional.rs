//! Zero or one occurrence

use crate::cursor::Cursor;
use crate::error::{Advance, ErrorNode};
use crate::parser::Parser;

/// Parser combinator that optionally matches the given parser
///
/// Returns `Some(value)` if the parser succeeds, and `None` without moving
/// the cursor if it fails without advancing ([`Advance::None`]); any other
/// failure is passed on (see [`Advance`]).
///
/// So `optional(p)` is not the same as `p.map(Some).or(default(None))`: `or`
/// also backtracks from a `p` that matched part of its input and then failed,
/// and drops that error.
#[must_use = "parsers do nothing until `.parse()` is called"]
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

    #[inline]
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

/// Create a parser that outputs `None` where `parser` matches nothing; see
/// [`Optional`]
pub fn optional<P>(parser: P) -> Optional<P>
where
    P: Parser,
{
    Optional::new(parser)
}

/// Extension trait to add .optional() method support for parsers
pub trait OptionalExt: Parser + Sized {
    /// Match this parser or nothing; see [`Optional`]
    fn optional(self) -> Optional<Self> {
        Optional::new(self)
    }
}

impl<P: Parser> OptionalExt for P {}

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
