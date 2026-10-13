//! Repeat a parser up to the end of the input

use crate::cursor::Cursor;
use crate::parser::Parser;

use crate::error::composite_error;

composite_error! {
    /// Error type for the All parser
    pub enum AllError<PErr, CurErr> {
        /// Error from the child parser
        ParserError(PErr),
        /// The child parser succeeded without consuming input before the end
        /// of input,
        /// so repeating it would never terminate
        NoProgress(CurErr),
    }
}

/// Parser combinator that repeats a parser up to the end of the input
///
/// The `All` combinator takes a parser and applies it repeatedly until either:
/// - The parser fails (returns the error)
/// - The end of the input is reached (returns all parsed items)
/// - The parser succeeds without consuming input (returns `AllError::NoProgress`)
///
/// This is useful for top-level parsers where you want to parse everything in the input.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct All<P> {
    parser: P,
}

impl<P> All<P> {
    /// Same as [`all`]
    pub fn new(parser: P) -> Self {
        All { parser }
    }
}

impl<P: Parser> Parser for All<P> {
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = AllError<P::Error, <P::Cursor as Cursor>::Error>;

    fn parse(&self, mut cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::new();

        while !cursor.eos() {
            let (value, next_cursor) = self.parser.parse(cursor).map_err(AllError::ParserError)?;

            if next_cursor.position() == cursor.position() {
                return Err(AllError::NoProgress(
                    cursor.invalid_error("parser succeeded without consuming input".into()),
                ));
            }

            results.push(value);
            cursor = next_cursor;
        }

        Ok((results, cursor))
    }
}

/// Create a parser that repeats `parser` up to the end of the input; see [`All`]
///
/// # Example
/// ```
/// use parsicomb::prelude::*;
/// use parsicomb::all;
/// use parsicomb::byte::is_byte;
///
/// let data = b"aaa";
/// let cursor = ByteCursor::new(data);
/// let parser = all(is_byte(b'a'));
///
/// // This will succeed and return vec![b'a', b'a', b'a']
/// let result = parser.parse(cursor);
/// assert!(result.is_ok());
/// ```
pub fn all<P: Parser>(parser: P) -> All<P> {
    All::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::byte::is_byte;
    use crate::cursor::Cursor;

    #[test]
    fn test_all_consumes_everything() {
        let data = b"aaaa";
        let cursor = ByteCursor::new(data);
        let parser = all(is_byte(b'a'));

        let result = parser.parse(cursor);
        assert!(result.is_ok());
        let (output, remaining) = result.unwrap();
        assert_eq!(output, vec![b'a', b'a', b'a', b'a']);
        assert!(remaining.eos());
    }

    #[test]
    fn test_all_stops_at_different_input() {
        let data = b"aaab";
        let cursor = ByteCursor::new(data);
        let parser = all(is_byte(b'a'));

        let result = parser.parse(cursor);
        // Should fail when it encounters 'b'
        assert!(result.is_err());
    }

    #[test]
    fn test_all_with_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = all(is_byte(b'a'));

        let result = parser.parse(cursor);
        assert!(result.is_ok());
        let (output, remaining) = result.unwrap();
        assert_eq!(output, vec![]);
        assert!(remaining.eos());
    }

    #[test]
    fn test_all_fails_immediately_on_wrong_input() {
        let data = b"b";
        let cursor = ByteCursor::new(data);
        let parser = all(is_byte(b'a'));

        let result = parser.parse(cursor);
        // Should fail immediately on 'b'
        assert!(result.is_err());
    }

    #[test]
    fn test_all_non_consuming_parser_errors() {
        let cursor = ByteCursor::new(b"b");
        let parser = all(crate::optional(is_byte(b'a')));

        let result = parser.parse(cursor);
        assert!(matches!(result, Err(AllError::NoProgress(_))));
    }

    #[test]
    fn test_no_progress_at_the_start_is_no_match() {
        use crate::combinator::MapExt;
        use crate::combinator::OrExt;
        use crate::combinator::optional;

        let parser = all(optional(is_byte(b'a')))
            .map(|_| 0)
            .or(is_byte(b'b').map(|_| 1));
        assert_eq!(parser.parse(ByteCursor::new(b"b")).unwrap().0, 1);
    }
}
