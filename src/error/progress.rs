//! How far a failed parser got

use super::Advance;
use std::cmp::Ordering;

/// How far a failed parser got
///
/// Every failure is somewhere, `at`: the message points there. The parsers
/// around a failed parser also need to know how far its progress counts, to
/// decide whether to backtrack ([`Progress::since`]). Usually that is `at`
/// too: a leaf builds its error at the cursor where it stopped, and
/// everything before that counts ([`Progress::Partial`]).
///
/// A parser can instead have only peeked at the input before failing, when
/// what it matched isn't evidence that it is on the right path
/// ([`Progress::Peek`]). A parser that skips whitespace before a token, then
/// finds no token, failed at the token but has only peeked past the
/// whitespace: the parsers around it see "nothing here" and try something
/// else, while the message, if it is reported, still points at the missing
/// token.
///
/// Errors that hold other errors pass their inner error's progress on,
/// possibly adjusted: committed, or turned into a peek.
///
/// The order says which of two failures from the same start tells more about
/// the input: a committed one, then the one whose progress counts further,
/// then the one that failed further in.
///
/// For example Parsec's `try`, where a failure counts as no progress so
/// enclosing combinators backtrack from it, turns its inner error's progress
/// into a peek from where it started, with [`WithProgress`](super::WithProgress):
///
/// ```
/// use parsicomb::{AndExt, ByteCursor, Cursor, ErrorNode, Parser, WithProgress, optional, parser_fn};
/// use parsicomb::byte::is_byte;
/// use parsicomb::utf8::is_string;
///
/// fn attempt<P: Parser>(
///     parser: P,
/// ) -> impl Parser<Cursor = P::Cursor, Output = P::Output, Error = WithProgress<P::Error>> {
///     parser_fn(move |cursor: P::Cursor| {
///         let start = cursor.position();
///         parser.parse(cursor).map_err(|error| {
///             // Still reported where it failed, but only peeked from the start
///             let progress = error.progress().peeked_from(start);
///             WithProgress::new(error, progress)
///         })
///     })
/// }
///
/// // `pub ` matches `pub`, then fails at `(`
/// let input = ByteCursor::new(b"pub(crate)");
/// let pub_space = || is_string("pub").and(is_byte(b' '));
/// assert!(optional(pub_space()).parse(input).is_err());
/// assert!(optional(attempt(pub_space())).parse(input).unwrap().0.is_none());
/// ```
///
/// Put a [`label`](crate::label()) inside such a wrapper: it only replaces a
/// failure at its start, and a peek fails further in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Progress {
    /// Failed at `at`, with progress up to it
    Partial {
        /// Where the failure is
        at: usize,
    },
    /// Failed at `at` after only peeking at the input from `from`: progress
    /// counts only up to `from` (`from < at`)
    Peek {
        /// Up to where the progress counts
        from: usize,
        /// Where the failure is
        at: usize,
    },
    /// Committed to this path before failing at `at`, or failed fatally
    /// (such as nesting too deep): nothing backtracks. A wrapper that turns
    /// it back into a plain failure undoes both, so it also makes a fatal
    /// failure recoverable
    Committed {
        /// Where the failure is
        at: usize,
    },
}

impl Progress {
    /// Failed at `position`, with progress up to it: the progress of a leaf
    /// error
    pub fn at(position: usize) -> Self {
        Progress::Partial { at: position }
    }

    /// Where the failure is in the input
    pub fn position(&self) -> usize {
        match *self {
            Progress::Partial { at } | Progress::Peek { at, .. } | Progress::Committed { at } => at,
        }
    }

    /// Up to where the progress counts, unless committed
    fn counts_to(&self) -> Option<usize> {
        match *self {
            Progress::Partial { at } => Some(at),
            Progress::Peek { from, .. } => Some(from),
            Progress::Committed { .. } => None,
        }
    }

    /// The same failure, having only peeked at the input from `from`;
    /// progress that counts only up to before `from` stays as it is, and a
    /// committed failure stays committed
    #[must_use = "returns the adjusted progress"]
    pub fn peeked_from(self, from: usize) -> Self {
        match self.counts_to() {
            Some(counts_to) if from < counts_to => Progress::Peek {
                from,
                at: self.position(),
            },
            _ => self,
        }
    }

    /// The same failure, committed
    #[must_use = "returns the adjusted progress"]
    pub fn committed(self) -> Self {
        Progress::Committed {
            at: self.position(),
        }
    }

    /// How far the failed parser advanced past `start`, where whoever ran it
    /// started it: [`Advance::Committed`] if committed, [`Advance::Partial`]
    /// if its progress counts past `start`, and [`Advance::None`] otherwise
    pub fn since(&self, start: usize) -> Advance {
        match self.counts_to() {
            None => Advance::Committed,
            Some(counts_to) if counts_to > start => Advance::Partial,
            Some(_) => Advance::None,
        }
    }
}

/// Committed first, then the progress that counts further, then the failure
/// further in
impl Ord for Progress {
    fn cmp(&self, other: &Self) -> Ordering {
        let key = |progress: &Progress| {
            (
                matches!(progress, Progress::Committed { .. }),
                progress.counts_to().unwrap_or(0),
                progress.position(),
            )
        };
        key(self).cmp(&key(other))
    }
}

impl PartialOrd for Progress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_is_committed_then_progress_then_position() {
        let peek = |from, at| Progress::Peek { from, at };
        assert!(Progress::Committed { at: 0 } > Progress::at(9));
        assert!(Progress::at(1) > peek(0, 9));
        assert!(peek(5, 9) > Progress::at(1));
        assert!(peek(0, 3) > peek(0, 2));
        assert!(Progress::at(3) > peek(0, 3));
        assert!(Progress::Committed { at: usize::MAX } > Progress::at(usize::MAX));
    }

    #[test]
    fn test_advance_depends_on_progress_past_start() {
        assert_eq!(Progress::at(2).since(2), Advance::None);
        assert_eq!(Progress::at(3).since(2), Advance::Partial);
        assert_eq!(Progress::at(3).peeked_from(2).since(2), Advance::None);
        assert_eq!(Progress::at(3).peeked_from(2).since(1), Advance::Partial);
        assert_eq!(Progress::at(2).committed().since(2), Advance::Committed);
    }

    #[test]
    fn test_peeking_only_lowers_progress() {
        assert_eq!(Progress::at(3).peeked_from(3), Progress::at(3));
        assert_eq!(Progress::at(3).peeked_from(4), Progress::at(3));
        let peek = Progress::at(5).peeked_from(2);
        assert_eq!(peek.peeked_from(3), peek);
        assert_eq!(peek.peeked_from(1), Progress::Peek { from: 1, at: 5 });
    }
}
