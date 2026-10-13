//! Delimited, separated lists such as `(a, b, c)`

use crate::cursor::Cursor;
use crate::error::{Advance, ErrorNode};
use crate::parser::Parser;

use crate::error::composite_error;

composite_error! {
    /// Error type for ArgumentList parser
    pub enum ArgumentListError<ErrOpen, ErrArg, ErrSep, ErrClose> {
        /// Failed to parse opening delimiter
        OpenDelimiter(ErrOpen),
        /// Failed to parse first argument
        FirstArgument(ErrArg),
        /// Failed to parse argument after separator
        ArgumentAfterSeparator(ErrArg),
        /// A separator failed after matching part of its input, or inside a
        /// `commit`
        Separator(ErrSep),
        /// Expected closing delimiter or separator
        ExpectedClosingDelimiter(ErrClose),
    }
}

/// Parser combinator for delimited argument lists
///
/// Parses: `open (arg (sep arg)*)? close`
///
/// # Examples
/// - `"()"` → `vec![]`
/// - `"(1)"` → `vec![1]`
/// - `"(1,2,3)"` → `vec![1, 2, 3]`
///
/// # Features
/// - Allows empty argument lists
/// - Distinguishes between missing delimiters and failed argument parsing
pub struct ArgumentList<POpen, PArg, PSep, PClose> {
    open: POpen,
    parser: PArg,
    separator: PSep,
    close: PClose,
}

impl<POpen, PArg, PSep, PClose> ArgumentList<POpen, PArg, PSep, PClose> {
    /// Same as [`argument_list`]
    pub fn new(open: POpen, parser: PArg, separator: PSep, close: PClose) -> Self {
        ArgumentList {
            open,
            parser,
            separator,
            close,
        }
    }
}

impl<POpen, PArg, PSep, PClose> Parser for ArgumentList<POpen, PArg, PSep, PClose>
where
    POpen: Parser,
    PArg: Parser<Cursor = POpen::Cursor>,
    PSep: Parser<Cursor = POpen::Cursor>,
    PClose: Parser<Cursor = POpen::Cursor>,
{
    type Cursor = POpen::Cursor;
    type Output = Vec<PArg::Output>;
    type Error = ArgumentListError<POpen::Error, PArg::Error, PSep::Error, PClose::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        // Parse opening delimiter
        let (_, cursor) = self
            .open
            .parse(cursor)
            .map_err(ArgumentListError::OpenDelimiter)?;

        // Check if we have an empty list (immediate closing delimiter)
        match self.close.parse(cursor) {
            Ok((_, cursor)) => return Ok((vec![], cursor)),
            Err(error) => match error.progress().since(cursor.position()) {
                Advance::None => {}
                Advance::Partial | Advance::Committed => {
                    return Err(ArgumentListError::ExpectedClosingDelimiter(error));
                }
            },
        }

        // Parse first argument (required if not empty)
        let (first, mut cursor) = self
            .parser
            .parse(cursor)
            .map_err(ArgumentListError::FirstArgument)?;

        let mut results = vec![first];

        // Parse remaining arguments preceded by separator
        loop {
            let sep_cursor = match self.separator.parse(cursor) {
                Ok((_, sep_cursor)) => sep_cursor,
                Err(error) => match error.progress().since(cursor.position()) {
                    Advance::None => break,
                    Advance::Partial | Advance::Committed => {
                        return Err(ArgumentListError::Separator(error));
                    }
                },
            };
            // Found separator, must parse another argument, unless the
            // separator matched nothing: then the arguments ended
            let (arg, next_cursor) = match self.parser.parse(sep_cursor) {
                Ok(result) => result,
                Err(error)
                    if sep_cursor.position() == cursor.position()
                        && error.progress().since(cursor.position()) == Advance::None =>
                {
                    break;
                }
                Err(error) => return Err(ArgumentListError::ArgumentAfterSeparator(error)),
            };

            // Separator + argument consumed nothing: stop rather than loop forever
            if next_cursor.position() == cursor.position() {
                break;
            }

            results.push(arg);
            cursor = next_cursor;
        }

        // No further separator, expect closing delimiter
        let (_, cursor) = self
            .close
            .parse(cursor)
            .map_err(ArgumentListError::ExpectedClosingDelimiter)?;
        Ok((results, cursor))
    }
}

/// Creates a parser for delimited argument lists
///
/// Parses: `open (arg (sep arg)*)? close`
///
/// Allows empty lists.
pub fn argument_list<POpen, PArg, PSep, PClose>(
    open: POpen,
    parser: PArg,
    separator: PSep,
    close: PClose,
) -> ArgumentList<POpen, PArg, PSep, PClose>
where
    POpen: Parser,
    PArg: Parser<Cursor = POpen::Cursor>,
    PSep: Parser<Cursor = POpen::Cursor>,
    PClose: Parser<Cursor = POpen::Cursor>,
{
    ArgumentList::new(open, parser, separator, close)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::ascii::number::i64;
    use crate::byte::is_byte;
    use crate::cursor::Cursor;

    #[test]
    fn test_empty_list() {
        let data = b"()";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![]);
        assert!(cursor.value().is_err()); // Consumed all input
    }

    #[test]
    fn test_single_argument() {
        let data = b"(42)";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![42]);
        assert!(cursor.value().is_err());
    }

    #[test]
    fn test_multiple_arguments() {
        let data = b"(1,2,3)";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![1, 2, 3]);
        assert!(cursor.value().is_err());
    }

    #[test]
    fn test_missing_opening_paren() {
        let data = b"1,2,3)";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
        // Should be OpenDelimiter error
        match result.unwrap_err() {
            ArgumentListError::OpenDelimiter(_) => (),
            _ => panic!("Expected OpenDelimiter error"),
        }
    }

    #[test]
    fn test_missing_closing_paren() {
        let data = b"(1,2,3";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
        // Should be ExpectedClosingDelimiter error
        match result.unwrap_err() {
            ArgumentListError::ExpectedClosingDelimiter(_) => (),
            e => panic!("Expected ExpectedClosingDelimiter error, got {:?}", e),
        }
    }

    #[test]
    fn test_first_argument_parse_failure() {
        let data = b"(abc)";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
        // Should be FirstArgument error
        match result.unwrap_err() {
            ArgumentListError::FirstArgument(_) => (),
            e => panic!("Expected FirstArgument error, got {:?}", e),
        }
    }

    #[test]
    fn test_argument_after_separator_failure() {
        let data = b"(1,abc)";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
        // Should be ArgumentAfterSeparator error
        match result.unwrap_err() {
            ArgumentListError::ArgumentAfterSeparator(_) => (),
            e => panic!("Expected ArgumentAfterSeparator error, got {:?}", e),
        }
    }

    #[test]
    fn test_trailing_separator() {
        let data = b"(1,2,)";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
        // Should be ArgumentAfterSeparator error (nothing after last comma)
        match result.unwrap_err() {
            ArgumentListError::ArgumentAfterSeparator(_) => (),
            e => panic!("Expected ArgumentAfterSeparator error, got {:?}", e),
        }
    }

    #[test]
    fn test_with_remaining_content() {
        let data = b"(1,2,3) extra";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(is_byte(b'('), i64(), is_byte(b','), is_byte(b')'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![1, 2, 3]);
        assert_eq!(cursor.value().unwrap(), b' ');
    }

    #[test]
    fn test_stops_when_separator_and_argument_consume_nothing() {
        use crate::optional::optional;

        let data = b"(b)";
        let cursor = ByteCursor::new(data);
        let parser = argument_list(
            is_byte(b'('),
            optional(is_byte(b'a')),
            optional(is_byte(b',')),
            is_byte(b')'),
        );

        // `b` is neither an argument nor the closing delimiter
        let result = parser.parse(cursor);
        assert!(matches!(
            result,
            Err(ArgumentListError::ExpectedClosingDelimiter(_))
        ));
    }
}
