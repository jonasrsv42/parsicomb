//! Bound recursion depth to avoid stack overflow on nested input

use crate::combinator::CommitError;
use crate::cursor::Cursor;
use crate::parser::Parser;
use std::cell::Cell;

thread_local! {
    /// Number of `MaxDepth` parsers currently running on this thread
    static DEPTH: Cell<usize> = const { Cell::new(0) };
}

use crate::error::composite_error;

composite_error! {
    /// Error type for MaxDepth parser
    pub enum MaxDepthError<PErr, TooDeepErr> {
        /// Error from the inner parser
        Inner(PErr),
        /// Parsing would nest deeper than the limit; committed, so enclosing
        /// `or`, `optional` and repetitions don't try something else
        TooDeep(TooDeepErr),
    }
}

/// Parser combinator that fails instead of nesting deeper than a limit
///
/// Recursive grammars (through `lazy`) use one stack frame chain per nesting
/// level, so input like `((((…` can overflow the stack and abort the process.
/// Wrapping the recursive parser in `max_depth` bounds that: all `MaxDepth`
/// parsers on a thread share one counter of how many of them are currently
/// running, and a parse that would exceed `max` fails with
/// [`MaxDepthError::TooDeep`] at the cursor where it would have started.
///
/// How much stack one level uses depends on the grammar, so pick `max` for
/// the stack size the parser runs on.
///
/// The counter is per thread rather than per parser because a recursive
/// grammar builds a new `MaxDepth` at every level (each `lazy` call builds
/// its own inner parser), so no single instance sees the whole depth. As a
/// result, `MaxDepth` levels of unrelated grammars add up when one runs
/// inside another on the same thread, e.g. a sub-language parsed in a `map`
/// closure: an inner grammar's `max` then counts the outer levels too. Use one
/// limit per grammar and size it for the combined nesting.
///
/// ```
/// use parsicomb::prelude::*;
/// use parsicomb::lazy;
/// use parsicomb::byte::is_byte;
///
/// fn nested<'code>()
/// -> impl Parser<Cursor = ByteCursor<'code>, Output = usize, Error = ParsicombError<'code>> {
///     is_byte(b'x')
///         .map(|_| 0)
///         .or(is_byte(b'(')
///             .and(lazy(|| nested().boxed()).max_depth(64))
///             .and(is_byte(b')'))
///             .map(|((_, depth), _)| depth + 1))
///         .map_err(ParsicombError::from)
/// }
///
/// assert!(nested().parse(ByteCursor::new(b"((x))")).is_ok());
///
/// let deep = vec![b'('; 100_000];
/// let error = nested().parse(ByteCursor::new(&deep)).unwrap_err();
/// assert!(error.to_string().contains("maximum nesting depth exceeded"));
/// ```
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct MaxDepth<P> {
    parser: P,
    max: usize,
}

impl<P> MaxDepth<P> {
    /// Same as [`max_depth`]
    pub fn new(parser: P, max: usize) -> Self {
        MaxDepth { parser, max }
    }
}

/// Decrements the thread's depth counter when a `MaxDepth` parse ends,
/// including by unwinding
struct DepthGuard;

impl Drop for DepthGuard {
    fn drop(&mut self) {
        DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}

impl<P: Parser> Parser for MaxDepth<P> {
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = MaxDepthError<P::Error, CommitError<<P::Cursor as Cursor>::Error>>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let current = DEPTH.with(Cell::get);
        if current >= self.max {
            return Err(MaxDepthError::TooDeep(CommitError::new(
                cursor.invalid_error("maximum nesting depth exceeded".into()),
            )));
        }
        DEPTH.with(|depth| depth.set(current + 1));
        let _guard = DepthGuard;

        self.parser.parse(cursor).map_err(MaxDepthError::Inner)
    }
}

/// Create a parser that fails instead of nesting deeper than `max` levels
pub fn max_depth<P: Parser>(parser: P, max: usize) -> MaxDepth<P> {
    MaxDepth::new(parser, max)
}

/// Extension trait to add .max_depth() method support for parsers
pub trait MaxDepthExt: Parser + Sized {
    /// Fail instead of nesting deeper than `max` levels; see [`MaxDepth`]
    fn max_depth(self, max: usize) -> MaxDepth<Self> {
        MaxDepth::new(self, max)
    }
}

impl<P: Parser> MaxDepthExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte::is_byte;
    use crate::combinator::AndExt;
    use crate::combinator::BoxedExt;
    use crate::combinator::MapErrExt;
    use crate::combinator::MapExt;
    use crate::combinator::OrExt;
    use crate::combinator::lazy;
    use crate::error::ErrorNode;
    use crate::{ByteCursor, ParsicombError};

    fn nested<'code>(
        max: usize,
    ) -> impl Parser<Cursor = ByteCursor<'code>, Output = usize, Error = ParsicombError<'code>>
    {
        is_byte(b'x')
            .map(|_| 0)
            .or(is_byte(b'(')
                .and(lazy(move || nested(max).boxed()).max_depth(max))
                .and(is_byte(b')'))
                .map(|((_, depth), _)| depth + 1))
            .map_err(ParsicombError::from)
    }

    #[test]
    fn test_within_limit() {
        let data = b"(((x)))";
        let (depth, cursor) = nested(3).parse(ByteCursor::new(data)).unwrap();
        assert_eq!(depth, 3);
        assert!(cursor.eos());
    }

    #[test]
    fn test_over_limit_fails_where_it_would_nest() {
        let data = b"((((x))))";
        let error = nested(3).parse(ByteCursor::new(data)).unwrap_err();
        assert_eq!(error.likely_error().loc().position(), 4);
        assert!(error.to_string().contains("maximum nesting depth exceeded"));
    }

    #[test]
    fn test_deep_input_does_not_overflow_the_stack() {
        let data = vec![b'('; 1_000_000];
        assert!(nested(64).parse(ByteCursor::new(&data)).is_err());
    }

    #[test]
    fn test_counter_is_restored_after_parse() {
        let data = b"((x))";
        for _ in 0..10 {
            assert!(nested(2).parse(ByteCursor::new(data)).is_ok());
            let _ = nested(2).parse(ByteCursor::new(b"(((x)))"));
        }
        assert_eq!(DEPTH.with(Cell::get), 0);
    }

    #[test]
    fn test_too_deep_is_not_swallowed_by_repetitions() {
        let data = b"(x)((((x))))";
        let error = crate::many(nested(3))
            .parse(ByteCursor::new(data))
            .unwrap_err();
        assert!(error.to_string().contains("maximum nesting depth exceeded"));
    }
}
