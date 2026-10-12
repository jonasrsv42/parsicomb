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

    /// What kind of failure this is: something expected that isn't there, or
    /// a real problem in the input
    ///
    /// Errors built by [`Cursor::expected_error`](crate::Cursor::expected_error)
    /// must return [`FailureKind::Expected`] and errors built by
    /// [`Cursor::invalid_error`](crate::Cursor::invalid_error) must return
    /// [`FailureKind::Invalid`]: whether a failure can be backtracked from
    /// depends on its kind (see [`Failure`]).
    fn kind(&self) -> FailureKind;
}

/// What kind of failure an error describes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// What was expected isn't there: "expected X, found Y", or the end of
    /// input
    Expected,
    /// A real problem in the input: a number too large, invalid UTF-8,
    /// nesting too deep
    Invalid,
}

/// How a failure looks to whoever started a parser at some position (see
/// [`FailureInfo::failure`])
///
/// Combinators that can recover from a failure, by trying something else or
/// stopping, decide by this: the more a failure says about the input, the
/// fewer combinators backtrack from it. The README's "How failures are
/// handled" lists what each built-in combinator does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// Nothing here: what was expected isn't at the start, and nothing that
    /// commits the parser was matched
    NoMatch,
    /// Something here is wrong: the parser matched part of its input before
    /// failing, or found a real problem ([`FailureKind::Invalid`])
    Failed,
    /// The parser committed to this path before failing: nothing backtracks
    Committed,
}

/// What is needed to decide about a failure, without walking the error
///
/// Built for a leaf with [`FailureInfo::of_leaf`]; errors that hold other
/// errors pass their inner error's info on, possibly adjusted (marked
/// committed, or with less of the input counting as consumed).
///
/// For example Parsec's `try`, where a failure counts as having consumed
/// nothing so enclosing combinators backtrack from it, is a wrapper whose
/// error adjusts `consumed`:
///
/// ```
/// use parsicomb::{ByteCursor, Consumed, Cursor, ErrorLeaf, ErrorNode, FailureInfo, Parser, optional};
/// use parsicomb::utf8::is_string;
/// use std::fmt;
///
/// struct Attempt<P>(P);
///
/// #[derive(Debug)]
/// struct AttemptError<E> {
///     inner: E,
///     start: usize,
/// }
///
/// impl<E: ErrorNode> fmt::Display for AttemptError<E> {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", self.inner.likely_error())
///     }
/// }
///
/// impl<E: ErrorNode> std::error::Error for AttemptError<E> {}
///
/// impl<E: ErrorNode> ErrorNode for AttemptError<E> {
///     type Element = E::Element;
///
///     fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
///         self.inner.likely_error()
///     }
///
///     // Still reported where it failed, but nothing counts as consumed
///     fn info(&self) -> FailureInfo {
///         self.inner.info().with_consumed(Consumed::UpTo(self.start))
///     }
/// }
///
/// impl<P: Parser> Parser for Attempt<P> {
///     type Cursor = P::Cursor;
///     type Output = P::Output;
///     type Error = AttemptError<P::Error>;
///
///     fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
///         let start = cursor.position();
///         self.0.parse(cursor).map_err(|inner| AttemptError { inner, start })
///     }
/// }
///
/// // `pub` matches the start of `push`, then fails
/// let input = ByteCursor::new(b"push");
/// assert!(optional(is_string("pub")).parse(input).is_err());
/// assert_eq!(optional(Attempt(is_string("pub"))).parse(input).unwrap().0, None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FailureInfo {
    /// Where the failure is in the input: the message points here, and of
    /// several failures the one that got furthest is reported
    pub position: usize,
    /// How much of the input the failed parser counts as having matched,
    /// which decides whether it matched anything (see
    /// [`failure`](Self::failure))
    pub consumed: Consumed,
    /// The kind of failure
    pub kind: FailureKind,
    /// The parser committed to this path before failing, so nothing
    /// backtracks from it
    pub committed: bool,
}

/// How much of the input a failed parser counts as having matched (see
/// [`FailureInfo::failure`])
///
/// Normally everything a parser matched before failing commits it to its
/// path. A parser can count less, when part of what it matched isn't
/// evidence that the path is right: input it skipped (such as whitespace),
/// or input it matched only tentatively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consumed {
    /// Everything up to where it failed: the usual case
    UpToFailure,
    /// Only up to this position; what it matched after this doesn't count
    UpTo(usize),
}

impl FailureInfo {
    /// The info of a leaf error: its position and kind, not committed
    pub fn of_leaf<L: ErrorLeaf + ?Sized>(leaf: &L) -> Self {
        FailureInfo {
            position: leaf.loc().position(),
            consumed: Consumed::UpToFailure,
            kind: leaf.kind(),
            committed: false,
        }
    }

    /// The same info with `consumed` replaced
    pub fn with_consumed(self, consumed: Consumed) -> Self {
        FailureInfo { consumed, ..self }
    }

    /// The same info with `committed` replaced
    pub fn with_committed(self, committed: bool) -> Self {
        FailureInfo { committed, ..self }
    }

    /// The position up to which the failed parser counts as having matched
    /// (see [`Consumed`])
    pub fn consumed_to(&self) -> usize {
        match self.consumed {
            Consumed::UpToFailure => self.position,
            Consumed::UpTo(position) => position,
        }
    }

    /// How this failure looks to whoever started the parser at `start`
    ///
    /// Committed failures are [`Failure::Committed`]. Otherwise a real
    /// problem, or an expectation after the parser consumed input past
    /// `start`, is [`Failure::Failed`]; an expectation with nothing consumed
    /// past `start` is [`Failure::NoMatch`].
    pub fn failure(&self, start: usize) -> Failure {
        if self.committed {
            Failure::Committed
        } else if self.kind == FailureKind::Invalid || self.consumed_to() > start {
            Failure::Failed
        } else {
            Failure::NoMatch
        }
    }
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
/// use parsicomb::error::{CodeLoc, ErrorLeaf, ErrorNode, FailureInfo, FailureKind};
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
///     // `Expected` for "expected X, found Y" errors
///     fn kind(&self) -> FailureKind {
///         FailureKind::Invalid
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
///     fn info(&self) -> FailureInfo {
///         FailureInfo::of_leaf(self)
///     }
/// }
/// ```
pub trait ErrorNode: Error {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// Flatten nested error structures and return the likely error that made it furthest
    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element>;

    /// [`FailureInfo`] of this error
    ///
    /// Combinators call this on every failure. A leaf error returns
    /// [`FailureInfo::of_leaf`]; an error that holds another error returns
    /// that error's info (otherwise a committed failure inside it is treated
    /// as recoverable), computed once rather than by walking a chain of errors
    /// on every call.
    fn info(&self) -> FailureInfo;

    /// How this failure looks to a combinator that started at `start`; see
    /// [`FailureInfo::failure`]
    fn failure(&self, start: usize) -> Failure {
        self.info().failure(start)
    }
}
