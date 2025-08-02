use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::error::{ErrorLeaf, ErrorNode};
use crate::parser::Parser;
use std::fmt;

/// Error type for Between parser that can wrap errors from all constituent parsers
pub enum BetweenError<'code, E1, E3, T: Atomic> {
    /// Error from the opening delimiter parser
    OpenDelimiter(E1),
    /// Error from the content parser (boxed to prevent type explosion)
    Content(Box<dyn ErrorNode<'code, Element = T> + 'code>),
    /// Error from the closing delimiter parser
    CloseDelimiter(E3),
}

impl<'code, E1, E3, T: Atomic> std::fmt::Debug for BetweenError<'code, E1, E3, T>
where
    E1: ErrorNode<'code, Element = T>,
    E3: ErrorNode<'code, Element = T>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BetweenError::OpenDelimiter(e) => f
                .debug_tuple("OpenDelimiter")
                .field(&format!("{}", e))
                .finish(),
            BetweenError::Content(e) => f
                .debug_tuple("Content")
                .field(&format!("{}", &**e))
                .finish(),
            BetweenError::CloseDelimiter(e) => f
                .debug_tuple("CloseDelimiter")
                .field(&format!("{}", e))
                .finish(),
        }
    }
}

impl<'code, E1: fmt::Display, E3: fmt::Display, T: Atomic> fmt::Display
    for BetweenError<'code, E1, E3, T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BetweenError::OpenDelimiter(e) => write!(f, "Open delimiter failed: {}", e),
            BetweenError::Content(e) => write!(f, "Content failed: {}", &**e),
            BetweenError::CloseDelimiter(e) => write!(f, "Close delimiter failed: {}", e),
        }
    }
}

impl<'code, E1, E3, T: Atomic> std::error::Error for BetweenError<'code, E1, E3, T>
where
    E1: ErrorNode<'code, Element = T>,
    E3: ErrorNode<'code, Element = T>,
{
}

impl<'code, E1, E3, T: Atomic + 'code> ErrorNode<'code> for BetweenError<'code, E1, E3, T>
where
    E1: ErrorNode<'code, Element = T>,
    E3: ErrorNode<'code, Element = T>,
{
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = Self::Element> {
        match self {
            BetweenError::OpenDelimiter(e1) => e1.likely_error(),
            BetweenError::Content(e2) => e2.as_ref().likely_error(),
            BetweenError::CloseDelimiter(e3) => e3.likely_error(),
        }
    }
}

/// Parser that matches content between opening and closing delimiters
///
/// This is a generic combinator that parses: `open + content + close`
/// and returns just the `content` value with the delimiters discarded.
///
/// Unlike the UTF-8 whitespace version, this does not handle whitespace automatically.
/// For whitespace handling, use the specific version in `utf8::whitespace::between`.
///
/// # Examples
/// - `"[content]"` → `"content"`
/// - `"(value)"` → `"value"`
/// - `"{data}"` → `"data"`
pub struct Between<'code, P1, P3, C, O, E2>
where
    C: Cursor<'code>,
    P1: Parser<'code, Cursor = C>,
    P3: Parser<'code, Cursor = C>,
{
    open: P1,
    content: Box<dyn Parser<'code, Cursor = C, Output = O, Error = E2> + 'code>,
    close: P3,
}

impl<'code, P1, P3, C, O, E2> Parser<'code> for Between<'code, P1, P3, C, O, E2>
where
    P1: Parser<'code, Cursor = C> + 'code,
    P1::Error: ErrorNode<'code, Element = C::Element>,
    P3: Parser<'code, Cursor = C>,
    P3::Error: ErrorNode<'code, Element = C::Element>,
    E2: ErrorNode<'code, Element = C::Element> + 'code,
    C: Cursor<'code>,
    C::Element: Atomic + 'code,
{
    type Cursor = C;
    type Output = O;
    type Error = BetweenError<'code, P1::Error, P3::Error, C::Element>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (_, cursor) = self
            .open
            .parse(cursor)
            .map_err(BetweenError::OpenDelimiter)?;
        let (content_val, cursor) = self
            .content
            .parse(cursor)
            .map_err(|e| BetweenError::Content(Box::new(e)))?;
        let (_, cursor) = self
            .close
            .parse(cursor)
            .map_err(BetweenError::CloseDelimiter)?;

        Ok((content_val, cursor))
    }
}

impl<'code, P1, P3, C, O, E2> Between<'code, P1, P3, C, O, E2>
where
    C: Cursor<'code>,
    P1: Parser<'code, Cursor = C>,
    P3: Parser<'code, Cursor = C>,
{
    pub fn new<P2>(open: P1, content: P2, close: P3) -> Self
    where
        P1::Error: ErrorNode<'code, Element = C::Element> + 'code,
        P2: Parser<'code, Cursor = C, Output = O, Error = E2> + 'code,
        P3::Error: ErrorNode<'code, Element = C::Element> + 'code,
        E2: ErrorNode<'code, Element = C::Element> + 'code,
        C::Element: Atomic + 'code,
    {
        Between {
            open,
            content: Box::new(content),
            close,
        }
    }
}

/// Creates a parser that matches content between opening and closing delimiters
///
/// This is a generic combinator that does not handle whitespace automatically.
/// For automatic whitespace handling, use `utf8::whitespace::between`.
pub fn between<'code, P1, P2, P3>(
    open: P1,
    content: P2,
    close: P3,
) -> Between<'code, P1, P3, P1::Cursor, P2::Output, P2::Error>
where
    P1: Parser<'code> + 'code,
    P1::Cursor: Cursor<'code>,
    P2: Parser<'code, Cursor = P1::Cursor> + 'code,
    P3: Parser<'code, Cursor = P1::Cursor> + 'code,
    P1::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element> + 'code,
    P2::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element> + 'code,
    P3::Error: ErrorNode<'code, Element = <P1::Cursor as Cursor<'code>>::Element> + 'code,
    <P1::Cursor as Cursor<'code>>::Element: Atomic + 'code,
{
    Between::new(open, content, close)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::ascii::number::f64;
    use crate::byte::is_byte;
    use crate::utf8::string::is_string;

    #[test]
    fn test_brackets_number() {
        let data = b"[42.5]";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - 42.5).abs() < f64::EPSILON);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_parentheses_string() {
        let data = b"(hello)";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'('), is_string("hello"), is_byte(b')'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert_eq!(value.as_ref(), "hello");
    }

    #[test]
    fn test_braces() {
        let data = b"{test}";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'{'), is_string("test"), is_byte(b'}'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert_eq!(value.as_ref(), "test");
    }

    #[test]
    fn test_missing_open_delimiter_fails() {
        let data = b"42.0]";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_missing_close_delimiter_fails() {
        let data = b"[42.0";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_with_remaining_content() {
        let data = b"[42.0] extra";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - 42.0).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b' ');
    }
}
