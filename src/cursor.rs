use std::error::Error;

/// Generic cursor trait for parser combinators
///
/// A cursor represents a position in a sequence of elements that can be advanced
/// and queried. This abstraction allows parsers to work with different underlying
/// data types (bytes, tokens, etc.) while maintaining the same combinator interface.
pub trait Cursor<'code>: Copy + Clone + Sized {
    /// The type of elements this cursor iterates over
    type Element;

    /// Error type returned when cursor operations fail
    type Error: Error;

    /// Get the element at the current cursor position
    ///
    /// Returns an error if the cursor is positioned at the end of the sequence
    fn value(&self) -> Result<Self::Element, Self::Error>;

    /// Advance the cursor to the next element
    ///
    /// If already at the end, returns a cursor still positioned at the end
    fn next(self) -> Self;

    /// Advance the cursor to the next element, returning an error if at end
    ///
    /// Unlike `next()`, this method returns an error if called when already
    /// at the end of the sequence
    fn try_next(self) -> Result<Self, Self::Error>;

    /// Get the current position in the sequence
    ///
    /// For end-of-sequence cursors, this typically returns the length of the sequence
    fn position(&self) -> usize;

    /// Check if the cursor is at the end of the sequence
    fn eos(&self) -> bool {
        self.value().is_err()
    }

    /// Get the source data without consuming the cursor
    fn source(&self) -> &'code [Self::Element];

    /// Consume the cursor and return its inner data and position
    ///
    /// Returns a tuple of (data_slice, current_position) where data_slice
    /// contains all the elements and current_position is the cursor's position
    fn inner(self) -> (&'code [Self::Element], usize);
}
