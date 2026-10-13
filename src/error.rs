//! Error types and the failure model
//!
//! # How failures are handled
//!
//! Every error says how far the failed parser got
//! ([`ErrorNode::progress`]). Each failure is somewhere (`at`), and the
//! message points there. The parsers around a failed parser also need to know
//! how far its progress counts, to decide whether to backtrack:
//!
//! - [`Progress::Partial`]: progress up to the failure. A leaf parser builds
//!   its error at the cursor where it stopped, with
//!   [`Cursor::expected_error`](crate::Cursor::expected_error) where what it
//!   expected isn't there, or
//!   [`Cursor::invalid_error`](crate::Cursor::invalid_error) after input that
//!   is there but wrong (a number too large, invalid UTF-8).
//! - [`Progress::Peek`]: it only peeked at the input from `from` before
//!   failing at `at`, so its progress counts only up to `from`. This is for
//!   input that isn't evidence the parser is on the right path:
//!   [`skip_then`](crate::skip_then()) doesn't count the whitespace it
//!   skipped, a literal that matched only partly doesn't count either, and
//!   [`attempt`](crate::attempt()) turns any failure inside it into a peek
//!   from its start.
//!   The parsers around it see "nothing here" while the message still points
//!   at the mismatch. Put [`label`](crate::label()) inside `skip_then` rather
//!   than around it, so the label replaces the error where the token is
//!   missing.
//! - [`Progress::Committed`]: it failed inside a
//!   [`commit`](crate::commit()), or fatally (nesting too deep).
//!
//! A combinator that started a parser at `start` sees how far it advanced
//! with [`Progress::since`]:
//!
//! - [`Advance::None`]: no progress past the start. Nothing here.
//! - [`Advance::Partial`]: progress past the start, then a failure. Something
//!   here is wrong.
//! - [`Advance::Committed`]: committed or fatal.
//!
//! Progress is ordered: committed first, then the progress that counts
//! further, then the failure further in. `or` reports the alternative with
//! the greater progress, so an alternative that matched nothing (and isn't
//! committed) is never reported over one that matched input, however far in
//! its error points.
//!
//! Leaf parsers that run out of input report `expected …, found end of
//! input`; reading past the end with
//! [`Cursor::value`](crate::Cursor::value) in a hand-written parser gives
//! `Unexpected end of input`.

use crate::atomic::Atomic;
use std::error::Error;

mod advance;
mod code_loc;
mod composite;
mod expected;
mod parsicomb;
mod progress;
pub(crate) mod render;
mod with_progress;

pub use advance::Advance;
pub use code_loc::CodeLoc;
pub(crate) use composite::composite_error;
pub use expected::Expected;
pub use parsicomb::ParsicombError;
pub use progress::Progress;
pub use with_progress::WithProgress;

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
