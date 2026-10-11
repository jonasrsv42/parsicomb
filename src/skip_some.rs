//! One or more repetitions without collecting outputs

use super::parser::Parser;
use crate::cursor::Cursor;
use crate::error::ErrorNode;

/// Parser combinator that matches one or more occurrences of the given
/// parser and discards their outputs
///
/// Like [`some`](crate::some::some) without collecting a `Vec`.
///
/// Stops where the parser matches nothing (see
/// [`ErrorRank::is_no_match_at`](crate::error::ErrorRank::is_no_match_at));
/// any other failure, one after the parser matched part of its input or a
/// real problem, is passed on.
pub struct SkipSome<P> {
    parser: P,
}

impl<P> SkipSome<P> {
    /// Same as [`skip_some`]
    pub fn new(parser: P) -> Self {
        SkipSome { parser }
    }
}

impl<P: Parser> Parser for SkipSome<P> {
    type Cursor = P::Cursor;
    type Output = ();
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        // First parse must succeed
        let (_, mut cursor) = self.parser.parse(cursor)?;

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

/// Convenience function to create a SkipSome parser
pub fn skip_some<P: Parser>(parser: P) -> SkipSome<P> {
    SkipSome::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::byte::is_byte;

    #[test]
    fn test_skips_all_matches() {
        let (_, cursor) = skip_some(is_byte(b'a'))
            .parse(ByteCursor::new(b"aab"))
            .unwrap();
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_zero_matches_fails() {
        let error = skip_some(is_byte(b'a'))
            .parse(ByteCursor::new(b"b"))
            .unwrap_err();
        assert_eq!(error.position(), 0);
    }
}
