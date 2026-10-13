//! Full backtracking: a failure counts as no progress

use crate::cursor::Cursor;
use crate::error::{ErrorNode, WithProgress};
use crate::parser::Parser;

/// Parser combinator whose failures count as no progress, so enclosing
/// `optional` and repetitions backtrack from them (Parsec's `try`)
///
/// The error is still reported where the inner parser failed, but it only
/// peeked at the input from where it started (see
/// [`Progress::Peek`](crate::Progress::Peek)). A committed failure inside
/// stays committed.
///
/// ```
/// use parsicomb::prelude::*;
/// use parsicomb::byte::is_byte;
/// use parsicomb::optional;
/// use parsicomb::utf8::is_string;
///
/// // `pub ` matches `pub`, then fails at `(`
/// let input = ByteCursor::new(b"pub(crate)");
/// let pub_space = || is_string("pub").and(is_byte(b' '));
///
/// // Something here is wrong: `optional` passes the failure on
/// assert!(optional(pub_space()).parse(input).is_err());
///
/// // Nothing here: `optional` backtracks to the start
/// let (found, rest) = optional(pub_space().attempt()).parse(input).unwrap();
/// assert!(found.is_none());
/// assert_eq!(rest.position(), 0);
/// ```
///
/// Put a [`label`](crate::label()) inside `attempt`: it only replaces a
/// failure at its start, and an attempted failure further in still points
/// there.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Attempt<P> {
    parser: P,
}

impl<P> Attempt<P> {
    /// Same as [`attempt`]
    pub fn new(parser: P) -> Self {
        Attempt { parser }
    }
}

impl<P: Parser> Parser for Attempt<P> {
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = WithProgress<P::Error>;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        self.parser.parse(cursor).map_err(|error| {
            let progress = error.progress().peeked_from(cursor.position());
            WithProgress::new(error, progress)
        })
    }
}

/// Create a parser whose failures count as no progress; see [`Attempt`]
pub fn attempt<P: Parser>(parser: P) -> Attempt<P> {
    Attempt::new(parser)
}

/// Extension trait to add .attempt() method support for parsers
pub trait AttemptExt: Parser + Sized {
    /// Make failures of this parser count as no progress; see [`Attempt`]
    fn attempt(self) -> Attempt<Self> {
        Attempt::new(self)
    }
}

impl<P: Parser> AttemptExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte::is_byte;
    use crate::combinator::{AndExt, CommitExt, MapExt, OrExt};
    use crate::error::{Advance, Progress};
    use crate::{ByteCursor, ParsicombError, many, optional};

    #[test]
    fn test_success_passes_through() {
        let (value, cursor) = attempt(is_byte(b'a').and(is_byte(b'b')))
            .parse(ByteCursor::new(b"abc"))
            .unwrap();
        assert_eq!(value, (b'a', b'b'));
        assert_eq!(cursor.position(), 2);
    }

    #[test]
    fn test_failure_is_a_peek_from_the_start() {
        let input = ByteCursor::new(b"xab!").advance(1);
        let error = is_byte(b'a')
            .and(is_byte(b'b'))
            .and(is_byte(b'c'))
            .attempt()
            .parse(input)
            .unwrap_err();
        assert_eq!(error.progress(), Progress::Peek { from: 1, at: 3 });
        assert_eq!(error.progress().since(1), Advance::None);
        assert!(
            error.to_string().contains("expected 'c', found '!'"),
            "got {error}"
        );
    }

    #[test]
    fn test_repetitions_stop_at_an_attempted_failure() {
        let pair = || is_byte(b'a').and(is_byte(b'b'));
        let input = ByteCursor::new(b"ababac");
        assert!(many(pair()).parse(input).is_err());

        let (items, rest) = many(pair().attempt()).parse(input).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(rest.position(), 4);
    }

    #[test]
    fn test_committed_failure_stays_committed() {
        let parser = is_byte(b'a').and(is_byte(b'b').commit()).attempt();
        let error = optional(parser).parse(ByteCursor::new(b"ac")).unwrap_err();
        assert_eq!(error.progress(), Progress::Committed { at: 1 });
    }

    #[test]
    fn test_converts_into_parsicomb_error_keeping_progress() {
        let parser = is_byte(b'a')
            .and(is_byte(b'b'))
            .attempt()
            .map(|_| 0)
            .or(is_byte(b'x').map(|_| 1));
        let error: ParsicombError = parser.parse(ByteCursor::new(b"ac")).unwrap_err().into();
        assert_eq!(error.progress(), Progress::Peek { from: 0, at: 1 });
    }
}
