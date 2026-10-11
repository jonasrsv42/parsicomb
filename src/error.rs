//! Error types and the furthest-error machinery

use crate::atomic::Atomic;
use std::error::Error;

mod code_loc;
mod composite;
mod expected;
mod parsicomb;
pub(crate) mod render;

pub use code_loc::CodeLoc;
pub(crate) use composite::composite_error;
pub use expected::Expected;
pub use parsicomb::ParsicombError;

/// Trait for errors that can report their location in the input
/// This enables selecting the error that progressed furthest when multiple parsers fail
pub trait ErrorLeaf: Error {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// Returns the location where this error occurred
    fn loc(&self) -> CodeLoc<'_, Self::Element>;

    /// Whether this error only says what was expected at its location, e.g.
    /// "expected X, found Y" or the end of input
    ///
    /// Such errors made no claim beyond "nothing here matched", so `label` may
    /// replace them with a description of the whole parser. Errors that
    /// describe a real problem (an overflow, invalid UTF-8, a depth limit)
    /// return `false` and are kept.
    ///
    /// Errors built by [`Cursor::expected_error`](crate::Cursor::expected_error)
    /// must return `true`; otherwise `label`, `filter` and `or` cannot tell
    /// them apart from real problems.
    fn is_expectation(&self) -> bool;
}

/// Where an error's [`likely_error`](ErrorNode::likely_error) is and whether
/// it is an expectation, used to compare errors without walking them
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorRank {
    /// Position of the likely error in the input
    pub position: usize,
    /// [`ErrorLeaf::is_expectation`] of the likely error
    pub is_expectation: bool,
}

/// Generic trait for error types that can be flattened to find the furthest error
///
/// This trait enables automatic furthest-error selection across all combinator types
/// (Or, And, Filter, etc.) by providing a way to flatten nested error structures
/// and find the error that made it furthest into the input.
///
/// # Example for downstream crates
///
/// ```rust
/// use parsicomb::error::{ErrorLeaf, ErrorNode, CodeLoc};
/// use std::error::Error;
/// use std::fmt;
///
/// // Your custom error type, borrowing the input it points into
/// #[derive(Debug)]
/// struct MyError<'code> {
///     code: &'code [u8],
///     position: usize,
///     message: String,
/// }
///
/// impl<'code> fmt::Display for MyError<'code> {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", self.message)
///     }
/// }
///
/// impl<'code> Error for MyError<'code> {}
///
/// impl<'code> ErrorLeaf for MyError<'code> {
///     type Element = u8;
///
///     fn loc(&self) -> CodeLoc<'_, Self::Element> {
///         CodeLoc::new(self.code, self.position)
///     }
///
///     // `true` for errors built by `Cursor::expected_error`
///     fn is_expectation(&self) -> bool {
///         false
///     }
/// }
///
/// // A leaf error is its own most likely error
/// impl<'code> ErrorNode for MyError<'code> {
///     type Element = u8;
///
///     fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
///         self
///     }
/// }
/// ```
pub trait ErrorNode: Error {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// Flatten nested error structures and return the likely error that made it furthest
    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element>;

    /// [`ErrorRank`] of [`likely_error`](ErrorNode::likely_error)
    ///
    /// `or` and `label` call this on every failure. Errors that hold a chain
    /// of other errors (such as a wrapped error) override it to return a rank
    /// computed once, so comparing errors stays cheap in deeply nested input.
    fn rank(&self) -> ErrorRank {
        let leaf = self.likely_error();
        ErrorRank {
            position: leaf.loc().position(),
            is_expectation: leaf.is_expectation(),
        }
    }
}
