//! Skip runs of elements matching a predicate

use crate::cursor::Cursor;
use crate::error::Expected;
use crate::parser::Parser;
use std::marker::PhantomData;

/// Advance past elements while `pred` holds, returning the cursor at the
/// first element that doesn't match (or at the end of input)
#[inline]
fn advance_while<Cur: Cursor>(mut cursor: Cur, pred: impl Fn(Cur::Element) -> bool) -> Cur {
    while let Ok(element) = cursor.value() {
        if !pred(element) {
            break;
        }
        cursor = cursor.next();
    }
    cursor
}

/// Parser that skips zero or more elements matching a predicate
///
/// Unlike `skip_many(atomic().filter(..))`, it checks each element directly
/// and builds no error where the run ends. Pair it with
/// [`recognize`](crate::recognize::recognize) to get the span it skipped.
pub struct SkipWhile<Cur, F> {
    pred: F,
    _cursor: PhantomData<Cur>,
}

impl<Cur, F> Parser for SkipWhile<Cur, F>
where
    Cur: Cursor,
    F: Fn(Cur::Element) -> bool,
{
    type Cursor = Cur;
    type Output = ();
    type Error = Cur::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        Ok(((), advance_while(cursor, &self.pred)))
    }
}

/// Create a parser that skips zero or more elements matching `pred`
pub fn skip_while<Cur, F>(pred: F) -> SkipWhile<Cur, F>
where
    Cur: Cursor,
    F: Fn(Cur::Element) -> bool,
{
    SkipWhile {
        pred,
        _cursor: PhantomData,
    }
}

/// Parser that skips one or more elements matching a predicate
///
/// Fails with `expected <label>, found …` when the first element doesn't
/// match.
pub struct SkipWhile1<Cur, F> {
    pred: F,
    label: &'static str,
    _cursor: PhantomData<Cur>,
}

impl<Cur, F> Parser for SkipWhile1<Cur, F>
where
    Cur: Cursor,
    F: Fn(Cur::Element) -> bool,
{
    type Cursor = Cur;
    type Output = ();
    type Error = Cur::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let end = advance_while(cursor, &self.pred);
        if end.position() == cursor.position() {
            return Err(cursor.expected_error(Expected::Label(self.label)));
        }
        Ok(((), end))
    }
}

/// Create a parser that skips one or more elements matching `pred`, described
/// by `label` in errors (e.g. "a digit")
pub fn skip_while1<Cur, F>(pred: F, label: &'static str) -> SkipWhile1<Cur, F>
where
    Cur: Cursor,
    F: Fn(Cur::Element) -> bool,
{
    SkipWhile1 {
        pred,
        label,
        _cursor: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recognize::recognize;
    use crate::{AtomicCursor, ByteCursor};

    #[test]
    fn test_skips_matching_run() {
        let digits = recognize(skip_while(|b: u8| b.is_ascii_digit()));
        let (span, cursor) = digits.parse(ByteCursor::new(b"123abc")).unwrap();
        assert_eq!(span.slice(), b"123");
        assert_eq!(cursor.position(), 3);
    }

    #[test]
    fn test_empty_run_succeeds() {
        let (_, cursor) = skip_while(|b: u8| b.is_ascii_digit())
            .parse(ByteCursor::new(b"abc"))
            .unwrap();
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn test_run_to_end_of_input() {
        let (_, cursor) = skip_while(|b: u8| b.is_ascii_digit())
            .parse(ByteCursor::new(b"123"))
            .unwrap();
        assert!(cursor.eos());
    }

    #[test]
    fn test_skip_while1_requires_one_match() {
        let digits = skip_while1(|b: u8| b.is_ascii_digit(), "a digit");
        assert_eq!(
            digits.parse(ByteCursor::new(b"12x")).unwrap().1.position(),
            2
        );

        let message = digits.parse(ByteCursor::new(b"x")).unwrap_err().to_string();
        assert!(
            message.contains("expected a digit, found byte 0x78 ('x')"),
            "got {message}"
        );
    }

    #[test]
    fn test_non_byte_elements() {
        let data = [2u32, 4, 5, 6];
        let evens = skip_while1(|n: u32| n.is_multiple_of(2), "an even number");
        let (_, cursor) = evens.parse(AtomicCursor::new(&data)).unwrap();
        assert_eq!(cursor.position(), 2);
    }
}
