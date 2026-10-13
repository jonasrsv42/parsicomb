//! Type erasure for parsers with `.boxed()`

use crate::cursor::Cursor;
use crate::error::ErrorNode;
use crate::parser::Parser;

/// A type-erased parser
///
/// Hides the concrete parser type behind a trait object, leaving only the
/// cursor, output and error types visible. Use it to:
/// - close a recursive grammar: `lazy(|| expr().boxed())`, where `expr`
///   returns a concrete error type (see [`lazy()`](crate::lazy()))
/// - cap type growth in long combinator chains; around 40–50 combinators in
///   one expression the compiler's recursion limit is reached, and compile
///   time grows steeply well before that
/// - store parsers of different concrete types in one collection
///
/// It costs one allocation at construction and a virtual call per parse, so
/// box at a few chosen boundaries.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct BoxedParser<'a, Cur, Out, Err> {
    parser: Box<dyn Parser<Cursor = Cur, Output = Out, Error = Err> + 'a>,
}

impl<'a, Cur, Out, Err> BoxedParser<'a, Cur, Out, Err> {
    /// Erase the type of `parser`
    pub fn new<P>(parser: P) -> Self
    where
        P: Parser<Cursor = Cur, Output = Out, Error = Err> + 'a,
    {
        BoxedParser {
            parser: Box::new(parser),
        }
    }
}

impl<'a, Cur, Out, Err> Parser for BoxedParser<'a, Cur, Out, Err>
where
    Cur: Cursor,
    Err: ErrorNode<Element = Cur::Element>,
{
    type Cursor = Cur;
    type Output = Out;
    type Error = Err;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        self.parser.parse(cursor)
    }
}

/// Extension trait adding `.boxed()` to every parser
pub trait BoxedExt: Parser + Sized {
    /// Erase this parser's type; see [`BoxedParser`]
    fn boxed<'a>(self) -> BoxedParser<'a, Self::Cursor, Self::Output, Self::Error>
    where
        Self: 'a,
    {
        BoxedParser::new(self)
    }
}

impl<P: Parser> BoxedExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte::is_byte;
    use crate::map::MapExt;
    use crate::{ByteCursor, ParsicombError};

    #[test]
    fn test_parsers_of_different_types_share_a_boxed_type() {
        let parsers: Vec<BoxedParser<ByteCursor, u8, ParsicombError>> =
            vec![is_byte(b'a').boxed(), is_byte(b'b').map(|b| b + 1).boxed()];

        let cursor = ByteCursor::new(b"b");
        assert!(parsers[0].parse(cursor).is_err());
        assert_eq!(parsers[1].parse(cursor).unwrap().0, b'c');
    }

    #[test]
    fn test_boxed_keeps_the_error() {
        let error = is_byte(b'a')
            .boxed()
            .parse(ByteCursor::new(b"x"))
            .unwrap_err();
        assert!(matches!(error, ParsicombError::Expected { .. }));
    }
}
