//! A parser after a prefix, keeping only the parser's output

use crate::and::AndError;
use crate::parser::Parser;

/// Parser combinator that matches `prefix` then `parser`, and returns only
/// `parser`'s output
///
/// The prefix's input counts as matched: unlike
/// [`skip_then`](crate::skip_then()), a failure after it is a failure past the
/// start.
///
/// ```
/// use parsicomb::{ByteCursor, Parser, PrecededExt};
/// use parsicomb::ascii::u64;
/// use parsicomb::byte::is_byte;
///
/// let id = u64().preceded_by(is_byte(b'#'));
/// assert_eq!(id.parse(ByteCursor::new(b"#42")).unwrap().0, 42);
/// ```
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Preceded<PPrefix, P> {
    prefix: PPrefix,
    parser: P,
}

impl<PPrefix, P> Preceded<PPrefix, P> {
    /// Same as [`preceded`]
    pub fn new(prefix: PPrefix, parser: P) -> Self {
        Preceded { prefix, parser }
    }
}

impl<PPrefix, P> Parser for Preceded<PPrefix, P>
where
    PPrefix: Parser,
    P: Parser<Cursor = PPrefix::Cursor>,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = AndError<PPrefix::Error, P::Error>;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (_, cursor) = self.prefix.parse(cursor).map_err(AndError::FirstParser)?;
        self.parser.parse(cursor).map_err(AndError::SecondParser)
    }
}

/// Create a parser that matches `prefix` then `parser`, keeping `parser`'s
/// output
pub fn preceded<PPrefix, P>(prefix: PPrefix, parser: P) -> Preceded<PPrefix, P>
where
    PPrefix: Parser,
    P: Parser<Cursor = PPrefix::Cursor>,
{
    Preceded::new(prefix, parser)
}

/// Extension trait to add .preceded_by() method support for parsers
pub trait PrecededExt: Parser + Sized {
    /// Match `prefix` before this parser, keeping only this parser's output
    fn preceded_by<PPrefix>(self, prefix: PPrefix) -> Preceded<PPrefix, Self>
    where
        PPrefix: Parser<Cursor = Self::Cursor>,
    {
        Preceded::new(prefix, self)
    }
}

impl<P: Parser> PrecededExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte::is_byte;
    use crate::error::{Advance, ErrorNode};
    use crate::{ByteCursor, Cursor};

    #[test]
    fn test_keeps_the_parser_output() {
        let parser = preceded(is_byte(b'-'), is_byte(b'x'));
        let (value, cursor) = parser.parse(ByteCursor::new(b"-x!")).unwrap();
        assert_eq!((value, cursor.position()), (b'x', 2));
    }

    #[test]
    fn test_a_failure_after_the_prefix_made_progress() {
        let parser = is_byte(b'x').preceded_by(is_byte(b'-'));
        let error = parser.parse(ByteCursor::new(b"-y")).unwrap_err();
        assert_eq!(error.progress().since(0), Advance::Partial);
        let error = parser.parse(ByteCursor::new(b"y")).unwrap_err();
        assert_eq!(error.progress().since(0), Advance::None);
    }
}
