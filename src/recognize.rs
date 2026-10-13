//! Output the span a parser matched instead of its output

use super::parser::Parser;
use crate::cursor::Cursor;

/// Parser combinator that outputs the span of input the given parser
/// matched, discarding its output
///
/// With [`skip_many`](crate::skip_many::skip_many) this matches runs of input
/// without collecting them, e.g. `recognize(skip_some(byte().filter(..)))`.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Recognize<P> {
    parser: P,
}

impl<P> Recognize<P> {
    /// Same as [`recognize`]
    pub fn new(parser: P) -> Self {
        Recognize { parser }
    }
}

impl<P: Parser> Parser for Recognize<P> {
    type Cursor = P::Cursor;
    type Output = <P::Cursor as Cursor>::Span;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (_, end) = self.parser.parse(cursor)?;
        Ok((cursor.span_to(end), end))
    }
}

/// Extension trait to add .recognize() method support for parsers
pub trait RecognizeExt: Parser + Sized {
    /// Output the span this parser matched instead of its output
    fn recognize(self) -> Recognize<Self> {
        Recognize::new(self)
    }
}

impl<P: Parser> RecognizeExt for P {}

/// Convenience function to create a Recognize parser
pub fn recognize<P: Parser>(parser: P) -> Recognize<P> {
    Recognize::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::byte::byte;
    use crate::filter::FilterExt;
    use crate::skip_some::skip_some;

    #[test]
    fn test_outputs_matched_span() {
        let digits = recognize(skip_some(byte().filter(|b| b.is_ascii_digit(), "digit")));
        let (span, cursor) = digits.parse(ByteCursor::new(b"123abc")).unwrap();
        assert_eq!(span.slice(), b"123");
        assert_eq!(cursor.position(), 3);
    }

    #[test]
    fn test_failure_is_passed_through() {
        let digits = skip_some(byte().filter(|b| b.is_ascii_digit(), "digit")).recognize();
        assert!(digits.parse(ByteCursor::new(b"abc")).is_err());
    }
}
