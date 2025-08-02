use crate::cursor::Cursor;
use crate::parser::Parser;

/// Represents a span in the source code with start and end positions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Start position (inclusive)
    pub start: usize,
    /// End position (exclusive)
    pub end: usize,
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// Get the length of the span
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if the span is empty
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// A parser combinator that captures the position span of a successful parse
pub struct Position<P> {
    parser: P,
}

impl<P> Position<P> {
    pub fn new(parser: P) -> Self {
        Position { parser }
    }
}

impl<'code, P> Parser<'code> for Position<P>
where
    P: Parser<'code>,
{
    type Cursor = P::Cursor;
    type Output = (P::Output, Span);
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let start_pos = cursor.position();
        let (output, new_cursor) = self.parser.parse(cursor)?;
        let end_pos = new_cursor.position();

        let span = Span::new(start_pos, end_pos);
        Ok(((output, span), new_cursor))
    }
}

/// Extension trait to add position tracking to any parser
pub trait PositionExt<'code>: Parser<'code> + Sized {
    /// Wrap this parser to capture its position span
    fn with_position(self) -> Position<Self> {
        Position::new(self)
    }
}

impl<'code, P> PositionExt<'code> for P where P: Parser<'code> {}

/// Convenience function to create a Position combinator
pub fn position<P>(parser: P) -> Position<P> {
    Position::new(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ByteCursor, Parser, byte::is_byte};

    #[test]
    fn test_span_basic() {
        let span = Span::new(0, 5);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_span_empty() {
        let span = Span::new(3, 3);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
    }

    #[test]
    fn test_position_single_byte() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = position(is_byte(b'h'));

        let ((byte, span), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'h');
        assert_eq!(span, Span::new(0, 1));
        assert_eq!(cursor.position(), 1);
    }

    #[test]
    fn test_position_extension_trait() {
        let data = b"world";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'w').with_position();

        let ((byte, span), _) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'w');
        assert_eq!(span, Span::new(0, 1));
    }

    #[test]
    fn test_position_multiple_bytes() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);

        // Parse first byte with position
        let parser = is_byte(b'a').with_position();
        let ((byte, span), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'a');
        assert_eq!(span, Span::new(0, 1));

        // Parse second byte with position
        let parser = is_byte(b'b').with_position();
        let ((byte, span), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'b');
        assert_eq!(span, Span::new(1, 2));

        // Parse third byte with position
        let parser = is_byte(b'c').with_position();
        let ((byte, span), _) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'c');
        assert_eq!(span, Span::new(2, 3));
    }

    #[test]
    fn test_position_with_multi_byte_parser() {
        use crate::utf8::string::is_string;

        let data = "hello world".as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello").with_position();

        let ((matched, span), _) = parser.parse(cursor).unwrap();
        assert_eq!(matched, "hello");
        assert_eq!(span, Span::new(0, 5));
    }

    #[test]
    fn test_position_error_propagation() {
        let data = b"xyz";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'a').with_position();

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }
}
