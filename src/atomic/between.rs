//! An element within an inclusive range

use crate::atomic::atomic;
use crate::cursor::Cursor;
use crate::error::Expected;
use crate::parser::Parser;
use std::marker::PhantomData;

/// Parser that matches an element within an inclusive range
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct AtomicBetween<Cur: Cursor> {
    start: Cur::Element,
    end: Cur::Element,
    _cursor: PhantomData<Cur>,
}

impl<Cur: Cursor> Parser for AtomicBetween<Cur>
where
    Cur::Element: PartialOrd,
{
    type Cursor = Cur;
    type Output = Cur::Element;
    type Error = Cur::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match atomic().parse(cursor) {
            Ok((value, next)) if self.start <= value && value <= self.end => Ok((value, next)),
            _ => Err(cursor.expected_error(Expected::Range(self.start, self.end))),
        }
    }
}

/// Create a parser that matches an element within an inclusive range
pub fn atomic_between<Cur: Cursor>(start: Cur::Element, end: Cur::Element) -> AtomicBetween<Cur>
where
    Cur::Element: PartialOrd,
{
    AtomicBetween {
        start,
        end,
        _cursor: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtomicCursor, ByteCursor};

    #[test]
    fn test_matches_inclusive_bounds() {
        let parser = atomic_between(b'0', b'9');
        assert_eq!(parser.parse(ByteCursor::new(b"0")).unwrap().0, b'0');
        assert_eq!(parser.parse(ByteCursor::new(b"9")).unwrap().0, b'9');
    }

    #[test]
    fn test_out_of_range_reports_range_and_found() {
        let data = [9u32];
        let parser = atomic_between(1u32, 5u32);
        let error = parser.parse(AtomicCursor::new(&data)).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("expected 1 to 5, found 9"),
            "got {message}"
        );
    }
}
