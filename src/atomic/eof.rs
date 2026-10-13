//! End of input

use crate::cursor::Cursor;
use crate::error::Expected;
use crate::parser::Parser;
use std::marker::PhantomData;

/// Parser that matches the end of the input, consuming nothing
///
/// Put it after a parser to require that it consumed the whole input:
///
/// ```
/// use parsicomb::{ByteCursor, Parser, TerminatedExt, eof};
/// use parsicomb::ascii::u64;
///
/// let number = u64().terminated_by(eof());
/// assert_eq!(number.parse(ByteCursor::new(b"42")).unwrap().0, 42);
///
/// let error = number.parse(ByteCursor::new(b"42x")).unwrap_err();
/// assert!(error.to_string().contains("expected end of input, found 'x'"));
/// ```
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Eof<Cur> {
    _cursor: PhantomData<Cur>,
}

impl<Cur: Cursor> Parser for Eof<Cur> {
    type Cursor = Cur;
    type Output = ();
    type Error = Cur::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        if cursor.eos() {
            Ok(((), cursor))
        } else {
            Err(cursor.expected_error(Expected::Label("end of input")))
        }
    }
}

/// Create a parser that matches the end of the input; see [`Eof`]
pub fn eof<Cur: Cursor>() -> Eof<Cur> {
    Eof {
        _cursor: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtomicCursor, ByteCursor};

    #[test]
    fn test_matches_at_end_without_consuming() {
        let input = ByteCursor::new(b"");
        let ((), cursor) = eof().parse(input).unwrap();
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn test_fails_where_input_remains() {
        let error = eof().parse(ByteCursor::new(b"ab").next()).unwrap_err();
        assert_eq!(error.position(), 1);
        assert!(
            error
                .to_string()
                .contains("expected end of input, found 'b'"),
            "got {error}"
        );
    }

    #[test]
    fn test_non_byte_elements() {
        let data = [1u32, 2];
        assert!(eof().parse(AtomicCursor::new(&data)).is_err());
        assert!(eof().parse(AtomicCursor::new(&data).advance(2)).is_ok());
    }
}
