//! Parser combinator that matches exactly N occurrences

use crate::cursor::Cursor;
use crate::error::composite_error;
use crate::parser::Parser;

composite_error! {
    /// Error type for the NTimes parser
    pub enum NTimesError<PErr, CurErr> {
        /// Error from the repeated parser
        ParserError(PErr),
        /// The repeated parser matched without consuming input and would be
        /// repeated again at the same place
        NoProgress(CurErr),
    }
}

/// Parser combinator that matches exactly N occurrences of the given parser
///
/// `count` may come from the input, so it is never used as a capacity, and a
/// match that consumes nothing is not repeated: if the parser matches without
/// consuming input and more repetitions remain, `ntimes` fails with
/// [`NTimesError::NoProgress`] instead of producing `count` identical matches.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct NTimes<P> {
    parser: P,
    count: usize,
}

impl<P> NTimes<P> {
    /// Same as [`ntimes`]
    pub fn new(parser: P, count: usize) -> Self {
        NTimes { parser, count }
    }
}

impl<P> Parser for NTimes<P>
where
    P: Parser,
{
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = NTimesError<P::Error, <P::Cursor as Cursor>::Error>;

    fn parse(&self, mut cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::new();

        for remaining in (0..self.count).rev() {
            let (value, next_cursor) = self
                .parser
                .parse(cursor)
                .map_err(NTimesError::ParserError)?;
            if remaining > 0 && next_cursor.position() == cursor.position() {
                return Err(NTimesError::NoProgress(cursor.invalid_error(
                    "repeated parser matched without consuming input".into(),
                )));
            }
            results.push(value);
            cursor = next_cursor;
        }

        Ok((results, cursor))
    }
}

/// Create a parser that matches `parser` exactly `count` times
pub fn ntimes<P>(parser: P, count: usize) -> NTimes<P>
where
    P: Parser,
{
    NTimes::new(parser, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::byte::is_byte;

    #[test]
    fn test_ntimes_zero() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), 0);

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, Vec::<u8>::new());
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_ntimes_one() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), 1);

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_ntimes_multiple() {
        let data = b"aaabcd";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), 3);

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_ntimes_exact_count() {
        let data = b"aaa";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), 3);

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
        assert!(cursor.eos());
    }

    #[test]
    fn test_ntimes_not_enough() {
        let data = b"aa";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), 3);

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_ntimes_wrong_byte() {
        let data = b"aab";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), 3);

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_ntimes_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), 1);

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_ntimes_empty_input_zero_count() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), 0);

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, Vec::<u8>::new());
    }

    #[test]
    fn test_ntimes_huge_count_fails_without_allocating_it() {
        let data = b"aaa";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(is_byte(b'a'), usize::MAX);

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_zero_width_match_is_not_repeated() {
        use crate::combinator::optional;

        let parser = ntimes(optional(is_byte(b'x')), 50_000_000);
        let message = parser
            .parse(ByteCursor::new(b"xy"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("without consuming input"), "got {message}");

        // A zero-width match as the last repetition is fine
        let (values, _) = ntimes(optional(is_byte(b'x')), 2)
            .parse(ByteCursor::new(b"xy"))
            .unwrap();
        assert_eq!(values, vec![Some(b'x'), None]);
    }
}
