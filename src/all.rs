use super::cursor::Cursor;
use super::parser::Parser;
use crate::error::{ErrorLeaf, ErrorNode};
use std::fmt;

/// Error type for the All parser
#[derive(Debug)]
pub enum AllError<PErr, CurErr> {
    /// Error from the child parser
    ParserError(PErr),
    /// The child parser succeeded without consuming input before end-of-stream,
    /// so repeating it would never terminate
    NoProgress(CurErr),
}

impl<PErr: fmt::Display, CurErr: fmt::Display> fmt::Display for AllError<PErr, CurErr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllError::ParserError(e) => write!(f, "{}", e),
            AllError::NoProgress(e) => write!(f, "{}", e),
        }
    }
}

impl<PErr: std::error::Error, CurErr: std::error::Error> std::error::Error
    for AllError<PErr, CurErr>
{
}

impl<PErr, CurErr> ErrorNode for AllError<PErr, CurErr>
where
    PErr: ErrorNode,
    CurErr: ErrorNode<Element = PErr::Element>,
{
    type Element = PErr::Element;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        match self {
            AllError::ParserError(e) => e.likely_error(),
            AllError::NoProgress(e) => e.likely_error(),
        }
    }
}

/// Parser combinator that repeatedly applies a parser until it fails or reaches end-of-stream
///
/// The `All` combinator takes a parser and applies it repeatedly until either:
/// - The parser fails (returns the error)
/// - End-of-stream is reached (returns all parsed items)
/// - The parser succeeds without consuming input (returns `AllError::NoProgress`)
///
/// This is useful for top-level parsers where you want to parse everything in the input.
pub struct All<P> {
    parser: P,
}

impl<P> All<P> {
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
                    cursor.syntax_error("parser succeeded without consuming input".into()),
                ));
            }

            results.push(value);
            cursor = next_cursor;
        }

        Ok((results, cursor))
    }
}

/// Convenience function to create an All parser
///
/// # Example
/// ```
/// use parsicomb::{all, byte::is_byte, ByteCursor, Parser};
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
}
