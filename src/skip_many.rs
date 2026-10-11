use super::parser::Parser;
use crate::cursor::Cursor;

/// Parser combinator that matches zero or more occurrences of the given
/// parser and discards their outputs
///
/// Like [`many`](crate::many::many) without collecting a `Vec`. Pair it with
/// [`recognize`](crate::recognize::recognize) to get the span it matched.
pub struct SkipMany<P> {
    parser: P,
}

impl<P> SkipMany<P> {
    pub fn new(parser: P) -> Self {
        SkipMany { parser }
    }
}

impl<P: Parser> Parser for SkipMany<P> {
    type Cursor = P::Cursor;
    type Output = ();
    type Error = P::Error;

    fn parse(&self, mut cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        while let Ok((_, next_cursor)) = self.parser.parse(cursor) {
            // An iteration that consumes nothing would repeat forever
            if next_cursor.position() == cursor.position() {
                break;
            }
            cursor = next_cursor;
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
