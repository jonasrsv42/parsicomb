use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::error::{ErrorLeaf, ErrorNode};
use crate::parser::Parser;
use std::fmt;

/// Error type for ArgumentList parser
#[derive(Debug)]
pub enum ArgumentListError<EOpen, EArg, ESep, EClose> {
    /// Failed to parse opening delimiter
    OpenDelimiter(EOpen),
    /// Failed to parse first argument
    FirstArgument(EArg),
    /// Failed to parse argument after separator
    ArgumentAfterSeparator(EArg),
    /// Expected closing delimiter or separator
    ExpectedClosingDelimiter(EClose),
    /// Separator error (internal use)
    Separator(ESep),
}

impl<EOpen, EArg, ESep, EClose> fmt::Display for ArgumentListError<EOpen, EArg, ESep, EClose>
where
    EOpen: fmt::Display,
    EArg: fmt::Display,
    ESep: fmt::Display,
    EClose: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgumentListError::OpenDelimiter(e) => {
                write!(f, "Failed to parse opening delimiter: {}", e)
            }
            ArgumentListError::FirstArgument(e) => {
                write!(f, "Failed to parse first argument: {}", e)
            }
            ArgumentListError::ArgumentAfterSeparator(e) => {
                write!(f, "Failed to parse argument after separator: {}", e)
            }
            ArgumentListError::ExpectedClosingDelimiter(e) => {
                write!(f, "Expected closing delimiter: {}", e)
            }
            ArgumentListError::Separator(e) => write!(f, "Separator error: {}", e),
        }
    }
}

impl<EOpen, EArg, ESep, EClose> std::error::Error for ArgumentListError<EOpen, EArg, ESep, EClose>
where
    EOpen: std::error::Error,
    EArg: std::error::Error,
    ESep: std::error::Error,
    EClose: std::error::Error,
{
}

impl<'code, EOpen, EArg, ESep, EClose, T: Atomic + 'code> ErrorNode<'code>
    for ArgumentListError<EOpen, EArg, ESep, EClose>
where
    EOpen: ErrorNode<'code, Element = T>,
    EArg: ErrorNode<'code, Element = T>,
    ESep: ErrorNode<'code, Element = T>,
    EClose: ErrorNode<'code, Element = T>,
{
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = T> {
        match self {
            ArgumentListError::OpenDelimiter(e) => e.likely_error(),
            ArgumentListError::FirstArgument(e) => e.likely_error(),
            ArgumentListError::ArgumentAfterSeparator(e) => e.likely_error(),
            ArgumentListError::ExpectedClosingDelimiter(e) => e.likely_error(),
            ArgumentListError::Separator(e) => e.likely_error(),
        }
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
/// - Provides context-specific error messages
/// - Distinguishes between missing delimiters and failed argument parsing
pub struct ArgumentList<POpen, PArg, PSep, PClose> {
    open: POpen,
    parser: PArg,
    separator: PSep,
    close: PClose,
}

impl<POpen, PArg, PSep, PClose> ArgumentList<POpen, PArg, PSep, PClose> {
    pub fn new(open: POpen, parser: PArg, separator: PSep, close: PClose) -> Self {
        ArgumentList {
            open,
            parser,
            separator,
            close,
        }
    }
}

impl<'code, POpen, PArg, PSep, PClose> Parser<'code> for ArgumentList<POpen, PArg, PSep, PClose>
where
    POpen: Parser<'code>,
    PArg: Parser<'code, Cursor = POpen::Cursor>,
    PSep: Parser<'code, Cursor = POpen::Cursor>,
    PClose: Parser<'code, Cursor = POpen::Cursor>,
    POpen::Cursor: Cursor<'code>,
    <POpen::Cursor as Cursor<'code>>::Element: Atomic + 'code,
    POpen::Error: ErrorNode<'code, Element = <POpen::Cursor as Cursor<'code>>::Element>,
    PArg::Error: ErrorNode<'code, Element = <POpen::Cursor as Cursor<'code>>::Element>,
    PSep::Error: ErrorNode<'code, Element = <POpen::Cursor as Cursor<'code>>::Element>,
    PClose::Error: ErrorNode<'code, Element = <POpen::Cursor as Cursor<'code>>::Element>,
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
        if let Ok((_, cursor)) = self.close.parse(cursor) {
            return Ok((vec![], cursor));
        }

        // Parse first argument (required if not empty)
        let (first, mut cursor) = self
            .parser
            .parse(cursor)
            .map_err(ArgumentListError::FirstArgument)?;

        let mut results = vec![first];

        // Parse remaining arguments preceded by separator
        loop {
            // Try to parse separator
            let sep_cursor = match self.separator.parse(cursor) {
                Ok((_, c)) => c,
                Err(_) => {
                    // No separator found, expect closing delimiter
                    let (_, cursor) = self
                        .close
                        .parse(cursor)
                        .map_err(ArgumentListError::ExpectedClosingDelimiter)?;
                    return Ok((results, cursor));
                }
            };

            // Found separator, must parse another argument
            let (arg, next_cursor) = self
                .parser
                .parse(sep_cursor)
                .map_err(ArgumentListError::ArgumentAfterSeparator)?;

            results.push(arg);
            cursor = next_cursor;
        }
    }
}

/// Creates a parser for delimited argument lists
///
/// Parses: `open (arg (sep arg)*)? close`
///
/// Allows empty lists and provides detailed error context.
pub fn argument_list<'code, POpen, PArg, PSep, PClose>(
    open: POpen,
    parser: PArg,
    separator: PSep,
    close: PClose,
) -> ArgumentList<POpen, PArg, PSep, PClose>
where
    POpen: Parser<'code>,
    PArg: Parser<'code, Cursor = POpen::Cursor>,
    PSep: Parser<'code, Cursor = POpen::Cursor>,
    PClose: Parser<'code, Cursor = POpen::Cursor>,
{
    ArgumentList::new(open, parser, separator, close)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::ascii::number::i64;
    use crate::byte::is_byte;

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
}
