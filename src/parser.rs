//! The [`Parser`] trait

use crate::cursor::Cursor;
use crate::error::ErrorNode;

/// Core parser trait for parser combinators
#[must_use = "parsers do nothing until `.parse()` is called"]
pub trait Parser {
    /// The cursor type this parser operates on
    type Cursor: Cursor;

    /// The output type produced by successful parsing
    type Output;

    /// The error type produced by failed parsing
    ///
    /// Combinators use the narrowest type that holds every failure they
    /// produce: the cursor's error when they only fail on their own (`not`),
    /// the inner parser's error when they only pass it on (`some`,
    /// `separated_list`, and `many`/`optional`, which fail only when a
    /// `commit` inside them fails), and an enum of both when they do both
    /// (`filter`, `all`, `max_depth`).
    type Error: ErrorNode<Element = <Self::Cursor as Cursor>::Element>;

    /// Attempt to parse from the given cursor position
    ///
    /// Returns Ok with the parsed value and updated cursor on success,
    /// or Err if the parse fails; on failure the caller keeps its original cursor.
    ///
    /// The returned cursor is never before `cursor` in the same input:
    /// combinators such as `recognize` and `with_position` build a span from
    /// the two and panic otherwise.
    ///
    /// Likewise an error's position is never before `cursor`: combinators
    /// decide whether anything matched by comparing the two (see
    /// [`Advance`](crate::error::Advance)).
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error>;
}

/// A reference to a parser is a parser, so one parser can be used in several
/// places without building it again: `item.and(&separator).and(&separator)`
impl<P: Parser + ?Sized> Parser for &P {
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        (**self).parse(cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::and::AndExt;
    use crate::byte::is_byte;
    use crate::{ByteCursor, many, separated_list};

    #[test]
    fn test_parser_used_by_reference_in_several_places() {
        let comma = is_byte(b',');
        let digit = crate::ascii::digit();
        let pair = (&digit).and(&comma).and(&digit);
        let ((a, _), b) = pair.parse(ByteCursor::new(b"1,2")).unwrap().0;
        assert_eq!((a, b), (b'1', b'2'));
        let list = separated_list(&digit, &comma);
        assert_eq!(list.parse(ByteCursor::new(b"1,2,3")).unwrap().0.len(), 3);
        assert_eq!(
            many(&comma).parse(ByteCursor::new(b",,x")).unwrap().0.len(),
            2
        );
    }
}
