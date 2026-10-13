//! Run a parser built from the previous parser's output

use crate::combinator::AndError;
use crate::parser::Parser;

/// Parser combinator that runs a parser, then the parser built from its output
///
/// For grammars where what comes next depends on what was parsed, e.g. a
/// closing delimiter that repeats the opening one.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct AndThen<P, F> {
    parser: P,
    next: F,
}

impl<P, F> AndThen<P, F> {
    /// Same as [`and_then`]
    pub fn new(parser: P, next: F) -> Self {
        AndThen { parser, next }
    }
}

impl<P, F, PNext> Parser for AndThen<P, F>
where
    P: Parser,
    F: Fn(P::Output) -> PNext,
    PNext: Parser<Cursor = P::Cursor>,
{
    type Cursor = P::Cursor;
    type Output = PNext::Output;
    type Error = AndError<P::Error, PNext::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (out, cursor) = self.parser.parse(cursor).map_err(AndError::FirstParser)?;
        (self.next)(out)
            .parse(cursor)
            .map_err(AndError::SecondParser)
    }
}

/// Extension trait to add .and_then() method support for parsers
pub trait AndThenExt: Parser + Sized {
    /// Run this parser, then the parser `next` builds from its output
    fn and_then<F, PNext>(self, next: F) -> AndThen<Self, F>
    where
        F: Fn(Self::Output) -> PNext,
        PNext: Parser<Cursor = Self::Cursor>,
    {
        AndThen::new(self, next)
    }
}

impl<P: Parser> AndThenExt for P {}

/// Convenience function to create an AndThen parser
pub fn and_then<P, F, PNext>(parser: P, next: F) -> AndThen<P, F>
where
    P: Parser,
    F: Fn(P::Output) -> PNext,
    PNext: Parser<Cursor = P::Cursor>,
{
    AndThen::new(parser, next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte::is_byte;
    use crate::combinator::MapExt;
    use crate::{ByteCursor, Cursor, ErrorNode, many, ntimes};

    /// `#`{n} then `x` then exactly n `#`
    fn balanced<'code>() -> impl Parser<Cursor = ByteCursor<'code>, Output = usize> {
        many(is_byte(b'#'))
            .map(|hashes| hashes.len())
            .and_then(|n| {
                is_byte(b'x')
                    .and_then(move |_| ntimes(is_byte(b'#'), n))
                    .map(move |_| n)
            })
    }

    #[test]
    fn test_next_parser_depends_on_output() {
        let (n, cursor) = balanced().parse(ByteCursor::new(b"##x##!")).unwrap();
        assert_eq!(n, 2);
        assert_eq!(cursor.value().unwrap(), b'!');
    }

    #[test]
    fn test_second_parser_failure_reports_its_location() {
        let error = balanced().parse(ByteCursor::new(b"##x#!")).unwrap_err();
        assert_eq!(error.likely_error().loc().position(), 4);
    }
}
