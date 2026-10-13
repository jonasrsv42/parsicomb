//! Error types and the failure model

use crate::atomic::Atomic;
use std::error::Error;

mod advance;
mod code_loc;
mod composite;
mod expected;
mod parsicomb;
mod progress;
pub(crate) mod render;

pub use advance::Advance;
pub use code_loc::CodeLoc;
pub(crate) use composite::composite_error;
pub use expected::Expected;
pub use parsicomb::ParsicombError;
pub use progress::Progress;

/// A single error: where in the input it is, and what to report
pub trait ErrorLeaf: Error {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// Returns the location where this error occurred
    fn loc(&self) -> CodeLoc<'_, Self::Element>;
}

/// An error of any parser, possibly holding the errors of the parsers it is
/// built from
///
/// It names the one leaf error to report ([`likely_error`](Self::likely_error))
/// and how far the parser got ([`progress`](Self::progress)), which
/// combinators decide on.
///
/// # Example for downstream crates
///
/// ```rust
/// use parsicomb::error::{CodeLoc, ErrorLeaf, ErrorNode, Progress};
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
/// }
///
/// // A leaf error is its own most likely error
/// impl<'code> ErrorNode for MyError<'code> {
///     type Element = u8;
///
///     fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
///         self
///     }
///
///     // Matched everything before where it failed
///     fn progress(&self) -> Progress {
///         Progress::at(self.position)
///     }
/// }
/// ```
pub trait ErrorNode: Error {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// The leaf error to report
    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element>;

    /// How far the failed parser got
    ///
    /// Combinators call this on every failure. A leaf error returns
    /// [`Progress::at`] its position; an error that holds another error
    /// returns that error's progress (otherwise a committed failure inside it
    /// is treated as recoverable). An error that holds a deep structure can
    /// compute it once and store it, as [`ParsicombError::wrap`] does.
    fn progress(&self) -> Progress;
}
