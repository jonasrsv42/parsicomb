//! Spans of matched input, and `with_position` to capture them

use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::parser::Parser;

/// Represents a span in the source code with start and end positions
/// and a reference to the source code
///
/// Two spans are equal when they cover the same range of the same input
/// (compared by address, not contents).
#[derive(Clone, Copy)]
pub struct Span<'code, T: Atomic = u8> {
    source: &'code [T],
    start: usize,
    end: usize,
}

impl<T: Atomic> PartialEq for Span<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.source, other.source)
            && self.start == other.start
            && self.end == other.end
    }
}

impl<T: Atomic> Eq for Span<'_, T> {}

/// Shows the range and input length, not the input
impl<T: Atomic> std::fmt::Debug for Span<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Span")
            .field("start", &self.start)
            .field("end", &self.end)
            .field("input_len", &self.source.len())
            .finish()
    }
}

impl<'code, T: Atomic> Span<'code, T> {
    /// Create a new span of `source[start..end]`
    ///
    /// # Panics
    ///
    /// If `start > end` or `end > source.len()`.
    pub fn new(source: &'code [T], start: usize, end: usize) -> Self {
        assert!(
            start <= end && end <= source.len(),
            "invalid span {start}..{end} of {} elements",
            source.len()
        );
        Span { source, start, end }
    }

    /// The full input this span points into
    pub fn source(&self) -> &'code [T] {
        self.source
    }

    /// Start position (inclusive)
    pub fn start(&self) -> usize {
        self.start
    }

    /// End position (exclusive)
    pub fn end(&self) -> usize {
        self.end
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
        self.source.get(self.start..self.end).unwrap_or_default()
    }

    /// Format the spanned content as a string
    pub fn as_string(&self) -> String {
        T::format_slice(self.slice())
    }
}

/// A parser combinator that captures the position span of a successful parse
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Position<P> {
    parser: P,
}

impl<P> Position<P> {
    /// Same as [`position`]
    pub fn new(parser: P) -> Self {
        Position { parser }
    }
}

impl<P: Parser> Parser for Position<P> {
    type Cursor = P::Cursor;
    type Output = (P::Output, <P::Cursor as Cursor>::Span);
    type Error = P::Error;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (output, new_cursor) = self.parser.parse(cursor)?;
        Ok(((output, cursor.span_to(new_cursor)), new_cursor))
    }
}

/// Extension trait to add position tracking to any parser
pub trait PositionExt: Parser + Sized {
    /// Wrap this parser to capture its position span
    fn with_position(self) -> Position<Self> {
        Position::new(self)
    }
}

impl<P: Parser> PositionExt for P {}

/// Create a parser that also outputs the span `parser` consumed; see
/// [`Position`]
pub fn position<P: Parser>(parser: P) -> Position<P> {
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
        use crate::utf8::is_string;

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
