//! Zero or more repetitions without collecting outputs

use super::parser::Parser;
use crate::cursor::Cursor;
use crate::error::ErrorNode;

/// Parser combinator that matches zero or more occurrences of the given
/// parser and discards their outputs
///
/// Like [`many`](crate::many::many) without collecting a `Vec`. Pair it with
/// [`recognize`](crate::recognize::recognize) to get the span it matched.
///
/// Stops where the parser matches nothing (see
/// [`ErrorRank::is_no_match_at`](crate::error::ErrorRank::is_no_match_at));
/// any other failure, one after the parser matched part of its input or a
/// real problem, is passed on.
pub struct SkipMany<P> {
    parser: P,
}

impl<P> SkipMany<P> {
    /// Same as [`skip_many`]
    pub fn new(parser: P) -> Self {
        SkipMany { parser }
    }
}

impl<P: Parser> Parser for SkipMany<P> {
    type Cursor = P::Cursor;
    type Output = ();
    type Error = P::Error;

    fn parse(&self, mut cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        loop {
            match self.parser.parse(cursor) {
                // An iteration that consumes nothing would repeat forever
                Ok((_, next_cursor)) if next_cursor.position() == cursor.position() => break,
                Ok((_, next_cursor)) => cursor = next_cursor,
                Err(error) if !error.rank().is_no_match_at(cursor.position()) => return Err(error),
                Err(_) => break,
            }
        }

        Ok(((), cursor))
    }
}

/// Convenience function to create a SkipMany parser
pub fn skip_many<P: Parser>(parser: P) -> SkipMany<P> {
    SkipMany::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::byte::is_byte;
    use crate::optional;

    #[test]
    fn test_skips_all_matches() {
        let (_, cursor) = skip_many(is_byte(b'a'))
            .parse(ByteCursor::new(b"aaab"))
            .unwrap();
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_zero_matches_succeeds() {
        let (_, cursor) = skip_many(is_byte(b'a'))
            .parse(ByteCursor::new(b"b"))
            .unwrap();
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn test_non_consuming_parser_terminates() {
        let (_, cursor) = skip_many(optional(is_byte(b'a')))
            .parse(ByteCursor::new(b"b"))
            .unwrap();
        assert_eq!(cursor.position(), 0);
    }
}
