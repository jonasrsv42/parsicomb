use super::cursor::Cursor;
use super::parser::Parser;
use crate::atomic::Atomic;
use crate::error::{CodeLoc, ErrorLeaf, ErrorNode, ParsicombError};
use std::fmt;

/// Error type for the All parser
#[derive(Debug)]
pub enum AllError<'code, E, T: Atomic = u8> {
    /// Error from the child parser
    ParserError(E),
    /// The child parser succeeded without consuming input before end-of-stream,
    /// so repeating it would never terminate
    NoProgress(ParsicombError<'code, T>),
}

impl<'code, E: fmt::Display, T: Atomic> fmt::Display for AllError<'code, E, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllError::ParserError(e) => write!(f, "{}", e),
            AllError::NoProgress(e) => write!(f, "{}", e),
        }
    }
}

impl<'code, E: std::error::Error, T: Atomic> std::error::Error for AllError<'code, E, T> {}

impl<'code, E, T: Atomic + 'code> ErrorNode<'code> for AllError<'code, E, T>
where
    E: ErrorNode<'code, Element = T>,
{
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = Self::Element> {
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

impl<'code, P> Parser<'code> for All<P>
where
    P: Parser<'code>,
    <P::Cursor as Cursor<'code>>::Element: Atomic + 'code,
{
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = AllError<'code, P::Error, <P::Cursor as Cursor<'code>>::Element>;

    fn parse(&self, mut cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::new();

        while !cursor.eos() {
            let (value, next_cursor) = self.parser.parse(cursor).map_err(AllError::ParserError)?;

            if next_cursor.position() == cursor.position() {
                let (data, position) = cursor.inner();
                return Err(AllError::NoProgress(ParsicombError::SyntaxError {
                    message: "parser succeeded without consuming input".into(),
                    loc: CodeLoc::new(data, position),
                }));
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
pub fn all<'code, P>(parser: P) -> All<P>
where
    P: Parser<'code>,
{
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
