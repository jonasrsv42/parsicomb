//! Parsers written as closures

use crate::cursor::Cursor;
use crate::error::ErrorNode;
use crate::parser::Parser;
use std::marker::PhantomData;

/// A parser defined by a closure from a cursor to a parse result
///
/// For custom parsers that are easiest written by hand, e.g. picking a rule
/// by the next element. Write out the type of the closure's cursor argument
/// (`|cursor: ByteCursor<'code>|`): the parser's cursor type is taken from it.
///
/// ```
/// use parsicomb::prelude::*;
/// use parsicomb::parser_fn;
/// use parsicomb::ascii::u64;
/// use parsicomb::byte::is_byte;
///
/// /// A number, or `-` for "none"; the next byte decides which rule runs
/// fn maybe_number<'code>()
/// -> impl Parser<Cursor = ByteCursor<'code>, Output = Option<u64>, Error = ParsicombError<'code>>
/// {
///     let number = u64().map(Some);
///     let none = is_byte(b'-').map(|_| None);
///     parser_fn(move |cursor: ByteCursor<'code>| match cursor.value()? {
///         b'-' => none.parse(cursor),
///         _ => number.parse(cursor),
///     })
/// }
///
/// assert_eq!(maybe_number().parse(ByteCursor::new(b"42")).unwrap().0, Some(42));
/// assert_eq!(maybe_number().parse(ByteCursor::new(b"-")).unwrap().0, None);
/// ```
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct ParserFn<Cur, F> {
    parse: F,
    _cursor: PhantomData<fn(Cur) -> Cur>,
}

impl<Cur, F, Out, Err> Parser for ParserFn<Cur, F>
where
    Cur: Cursor,
    F: Fn(Cur) -> Result<(Out, Cur), Err>,
    Err: ErrorNode<Element = Cur::Element>,
{
    type Cursor = Cur;
    type Output = Out;
    type Error = Err;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        (self.parse)(cursor)
    }
}

/// Create a parser from a closure from a cursor to a parse result
pub fn parser_fn<Cur, F, Out, Err>(parse: F) -> ParserFn<Cur, F>
where
    Cur: Cursor,
    F: Fn(Cur) -> Result<(Out, Cur), Err>,
    Err: ErrorNode<Element = Cur::Element>,
{
    ParserFn {
        parse,
        _cursor: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ByteCursor, ParsicombError};

    #[test]
    fn test_closure_parses() {
        let two = parser_fn(|cursor: ByteCursor| {
            let (a, cursor) = crate::byte::byte().parse(cursor)?;
            let (b, cursor) = crate::byte::byte().parse(cursor)?;
            Ok::<_, ParsicombError>(((a, b), cursor))
        });
        let ((a, b), cursor) = two.parse(ByteCursor::new(b"xyz")).unwrap();
        assert_eq!((a, b), (b'x', b'y'));
        assert_eq!(cursor.position(), 2);
    }
}
