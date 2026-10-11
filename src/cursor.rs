use crate::atomic::Atomic;
use crate::error::{ErrorNode, Expected};
use std::borrow::Cow;

/// Generic cursor trait for parser combinators
///
/// A cursor represents a position in a sequence of elements that can be advanced
/// and queried. This abstraction allows parsers to work with different underlying
/// data types (bytes, tokens, etc.) while maintaining the same combinator interface.
///
/// The cursor type carries the lifetime of the input it borrows (e.g.
/// `ByteCursor<'code>`). Values that borrow the input, such as errors and
/// spans, are produced through its associated `Error` and `Span` types.
pub trait Cursor: Copy {
    /// The type of elements this cursor iterates over
    type Element: Atomic;

    /// Error type for failures located at a cursor position
    ///
    /// Leaf parsers and combinators that report their own failures (e.g.
    /// `not`, `filter`, `all`) build it through `syntax_error` and
    /// `expected_error`.
    type Error: ErrorNode<Element = Self::Element>;

    /// A range of the input between two cursor positions
    type Span;

    /// Get the element at the current cursor position
    ///
    /// Returns an error if the cursor is positioned at the end of the sequence
    fn value(&self) -> Result<Self::Element, Self::Error>;

    /// Advance the cursor to the next element
    ///
    /// If already at the end, returns a cursor still positioned at the end
    fn next(self) -> Self;

    /// Advance the cursor, returning an error if the result would be at the
    /// end of the sequence or if the cursor is already there
    fn try_next(self) -> Result<Self, Self::Error>;

    /// Get the current position in the sequence
    ///
    /// For end-of-sequence cursors, this typically returns the length of the sequence
    fn position(&self) -> usize;

    /// Check if the cursor is at the end of the sequence
    fn eos(&self) -> bool {
        self.value().is_err()
    }

    /// Create an error located at this cursor with a free-form message
    fn syntax_error(&self, message: Cow<'static, str>) -> Self::Error;

    /// Create an error located at this cursor for a leaf parser that found
    /// something other than `expected`
    fn expected_error(&self, expected: Expected<Self::Element>) -> Self::Error;

    /// The span of input from this cursor up to (not including) `end`
    ///
    /// `end` must not be before this cursor.
    fn span_to(&self, end: Self) -> Self::Span;
}
