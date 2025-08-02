use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::parser::Parser;

/// Represents a span in the source code with start and end positions
/// and a reference to the source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span<'code, T: Atomic = u8> {
    /// Reference to the source code
    pub source: &'code [T],
    /// Start position (inclusive)
    pub start: usize,
    /// End position (exclusive)
    pub end: usize,
}

impl<'code, T: Atomic> Span<'code, T> {
    /// Create a new span
    pub fn new(source: &'code [T], start: usize, end: usize) -> Self {
        Span { source, start, end }
    }

    /// Get the length of the span
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if the span is empty
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Get the slice of code that this span represents
    pub fn slice(&self) -> &'code [T] {
        &self.source[self.start..self.end]
    }

    /// Format the spanned content as a string
    pub fn as_string(&self) -> String {
        T::format_slice(self.slice())
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
    P::Cursor: Cursor<'code>,
    <P::Cursor as Cursor<'code>>::Element: Atomic + 'code,
{
    type Cursor = P::Cursor;
    type Output = (
        P::Output,
        Span<'code, <P::Cursor as Cursor<'code>>::Element>,
    );
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let start_pos = cursor.position();
        let source = cursor.source();
        let (output, new_cursor) = self.parser.parse(cursor)?;
        let end_pos = new_cursor.position();

        let span = Span::new(source, start_pos, end_pos);
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
        let data = b"hello";
        let span = Span::new(data, 0, 5);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
        assert_eq!(span.slice(), b"hello");
        assert_eq!(span.as_string(), "hello");
    }

    #[test]
    fn test_span_empty() {
        let data = b"hello";
        let span = Span::new(data, 3, 3);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
        assert_eq!(span.slice(), b"");
        assert_eq!(span.as_string(), "");
    }

    #[test]
    fn test_span_slice() {
        let data = b"hello world";
        let span = Span::new(data, 6, 11);
        assert_eq!(span.slice(), b"world");
        assert_eq!(span.as_string(), "world");
    }

    #[test]
    fn test_position_single_byte() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = position(is_byte(b'h'));

        let ((byte, span), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'h');
        assert_eq!(span, Span::new(data, 0, 1));
        assert_eq!(span.slice(), b"h");
        assert_eq!(span.as_string(), "h");
        assert_eq!(cursor.position(), 1);
    }

    #[test]
    fn test_position_extension_trait() {
        let data = b"world";
        let cursor = ByteCursor::new(data);
        let parser = is_byte(b'w').with_position();

        let ((byte, span), _) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'w');
        assert_eq!(span, Span::new(data, 0, 1));
        assert_eq!(span.slice(), b"w");
    }

    #[test]
    fn test_position_multiple_bytes() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);

        // Parse first byte with position
        let parser = is_byte(b'a').with_position();
        let ((byte, span), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'a');
        assert_eq!(span, Span::new(data, 0, 1));
        assert_eq!(span.slice(), b"a");

        // Parse second byte with position
        let parser = is_byte(b'b').with_position();
        let ((byte, span), cursor) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'b');
        assert_eq!(span, Span::new(data, 1, 2));
        assert_eq!(span.slice(), b"b");

        // Parse third byte with position
        let parser = is_byte(b'c').with_position();
        let ((byte, span), _) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'c');
        assert_eq!(span, Span::new(data, 2, 3));
        assert_eq!(span.slice(), b"c");
    }

    #[test]
    fn test_position_with_multi_byte_parser() {
        use crate::utf8::string::is_string;

        let data = "hello world".as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = is_string("hello").with_position();

        let ((matched, span), _) = parser.parse(cursor).unwrap();
        assert_eq!(matched, "hello");
        assert_eq!(span, Span::new(data, 0, 5));
        assert_eq!(span.slice(), b"hello");
        assert_eq!(span.as_string(), "hello");
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
