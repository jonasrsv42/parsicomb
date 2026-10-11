//! A parser surrounded by opening and closing delimiters

use crate::parser::Parser;

use crate::error::composite_error;

composite_error! {
    /// Error type for Between parser that can wrap errors from all constituent parsers
    pub enum BetweenError<ErrOpen, ErrContent, ErrClose> {
        /// Error from the opening delimiter parser
        OpenDelimiter(ErrOpen),
        /// Error from the content parser
        Content(ErrContent),
        /// Error from the closing delimiter parser
        CloseDelimiter(ErrClose),
    }
}

/// Parser that matches content between opening and closing delimiters
///
/// This is a generic combinator that parses: `open + content + close`
/// and returns just the `content` value with the delimiters discarded.
/// It does not handle whitespace.
///
/// # Examples
/// - `"[content]"` → `"content"`
/// - `"(value)"` → `"value"`
/// - `"{data}"` → `"data"`
pub struct Between<POpen, PContent, PClose> {
    open: POpen,
    content: PContent,
    close: PClose,
}

impl<POpen, PContent, PClose> Between<POpen, PContent, PClose> {
    /// Same as [`between`]
    pub fn new(open: POpen, content: PContent, close: PClose) -> Self {
        Between {
            open,
            content,
            close,
        }
    }
}

impl<POpen, PContent, PClose> Parser for Between<POpen, PContent, PClose>
where
    POpen: Parser,
    PContent: Parser<Cursor = POpen::Cursor>,
    PClose: Parser<Cursor = POpen::Cursor>,
{
    type Cursor = POpen::Cursor;
    type Output = PContent::Output;
    type Error = BetweenError<POpen::Error, PContent::Error, PClose::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (_, cursor) = self
            .open
            .parse(cursor)
            .map_err(BetweenError::OpenDelimiter)?;
        let (content, cursor) = self.content.parse(cursor).map_err(BetweenError::Content)?;
        let (_, cursor) = self
            .close
            .parse(cursor)
            .map_err(BetweenError::CloseDelimiter)?;

        Ok((content, cursor))
    }
}

/// Creates a parser that matches content between opening and closing delimiters
///
/// This is a generic combinator that does not handle whitespace.
pub fn between<POpen, PContent, PClose>(
    open: POpen,
    content: PContent,
    close: PClose,
) -> Between<POpen, PContent, PClose>
where
    POpen: Parser,
    PContent: Parser<Cursor = POpen::Cursor>,
    PClose: Parser<Cursor = POpen::Cursor>,
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
