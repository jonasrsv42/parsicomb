//! Commit to a parse: failures after a commit point are not backtracked

use crate::atomic::Atomic;
use crate::error::{ErrorLeaf, ErrorNode, ParsicombError, Progress};
use crate::parser::Parser;
use std::fmt;

/// Error of a [`Commit`] parser: the inner parser's error, marked committed
///
/// Its [`progress`](ErrorNode::progress) is committed, so `or`, `optional` and
/// repetitions pass it on instead of trying something else.
#[derive(Debug)]
pub struct CommitError<E> {
    inner: E,
}

impl<E> CommitError<E> {
    pub(crate) fn new(inner: E) -> Self {
        CommitError { inner }
    }

    /// The inner parser's error
    pub fn into_inner(self) -> E {
        self.inner
    }
}

impl<E: ErrorNode> fmt::Display for CommitError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.likely_error())
    }
}

impl<E: ErrorNode> std::error::Error for CommitError<E> {}

impl<E: ErrorNode> ErrorNode for CommitError<E> {
    type Element = E::Element;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        self.inner.likely_error()
    }

    fn progress(&self) -> Progress {
        self.inner.progress().committed()
    }
}

/// Converts by wrapping (one allocation), which keeps the error committed
impl<'code, E, T> From<CommitError<E>> for ParsicombError<'code, T>
where
    E: ErrorNode<Element = T> + Send + Sync + 'code,
    T: Atomic,
{
    fn from(error: CommitError<E>) -> Self {
        ParsicombError::wrap(error)
    }
}

/// Parser combinator that commits: if the inner parser fails, the failure is
/// final, so enclosing `or`, `optional` and repetitions don't backtrack past it
///
/// Commit where the input can only mean one thing, typically right after a
/// keyword or opening delimiter has matched. `or` then doesn't try its
/// remaining alternatives, and `optional` and repetitions pass the failure on
/// even where it matched nothing:
///
/// ```
/// use parsicomb::{AndExt, ByteCursor, CommitExt, MapExt, OrExt, Parser};
/// use parsicomb::ascii::u64;
/// use parsicomb::byte::is_byte;
///
/// let input = ByteCursor::new(b"#x");
/// let bare = || is_byte(b'#').map(|_| 0);
///
/// // `or` falls back to a bare `#` after `#` and no number
/// let tagged = is_byte(b'#').and(u64()).map(|(_, n)| n);
/// assert_eq!(tagged.or(bare()).parse(input).unwrap().0, 0);
///
/// // Committed after `#`, a number is required
/// let tagged = is_byte(b'#').and(u64().commit()).map(|(_, n)| n);
/// let error = tagged.or(bare()).parse(input).unwrap_err();
/// assert!(error.to_string().contains("expected a digit, found 'x'"));
/// ```
///
/// Lookahead (`not`) and `label` pass committed failures on as they are.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Commit<P> {
    parser: P,
}

impl<P> Commit<P> {
    /// Same as [`commit`]
    pub fn new(parser: P) -> Self {
        Commit { parser }
    }
}

impl<P: Parser> Parser for Commit<P> {
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = CommitError<P::Error>;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        self.parser
            .parse(cursor)
            .map_err(|inner| CommitError { inner })
    }
}

/// Create a parser whose failures are committed; see [`Commit`]
pub fn commit<P: Parser>(parser: P) -> Commit<P> {
    Commit::new(parser)
}

/// Extension trait to add .commit() method support for parsers
pub trait CommitExt: Parser + Sized {
    /// Make failures of this parser committed; see [`Commit`]
    fn commit(self) -> Commit<Self> {
        Commit::new(self)
    }
}

impl<P: Parser> CommitExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::and::AndExt;
    use crate::boxed::BoxedExt;
    use crate::byte::is_byte;
    use crate::label::LabelExt;
    use crate::map::MapExt;
    use crate::map_err::MapErrExt;
    use crate::not::not;
    use crate::or::OrExt;
    use crate::{ByteCursor, Cursor, lazy, many, optional, separated_list, skip_many, some};

    /// `ab`, committed after `a`
    fn committed_ab<'code>()
    -> impl Parser<Cursor = ByteCursor<'code>, Output = u8, Error = ParsicombError<'code>> {
        is_byte(b'a')
            .and(is_byte(b'b').commit())
            .map(|(_, b)| b)
            .map_err(ParsicombError::from)
    }

    #[test]
    fn test_success_passes_through() {
        let (value, cursor) = committed_ab().parse(ByteCursor::new(b"abc")).unwrap();
        assert_eq!(value, b'b');
        assert_eq!(cursor.position(), 2);
    }

    #[test]
    fn test_failure_before_the_cut_backtracks() {
        let parser = committed_ab().or(is_byte(b'x'));
        assert_eq!(parser.parse(ByteCursor::new(b"x")).unwrap().0, b'x');
    }

    #[test]
    fn test_or_does_not_try_the_second_alternative_after_a_cut() {
        let parser = committed_ab().or(is_byte(b'a'));
        let error = parser.parse(ByteCursor::new(b"ac")).unwrap_err();
        assert!(error.progress().since(0) == crate::Advance::Committed);
        assert_eq!(error.progress().position(), 1);
    }

    #[test]
    fn test_committed_second_alternative_wins_over_a_further_first() {
        let parser = is_byte(b'a')
            .and(is_byte(b'a'))
            .and(is_byte(b'a'))
            .map(|_| 0)
            .or(committed_ab());
        let error = parser.parse(ByteCursor::new(b"aac")).unwrap_err();
        assert!(error.progress().since(0) == crate::Advance::Committed);
        assert_eq!(error.progress().position(), 1);
    }

    #[test]
    fn test_optional_and_repetitions_pass_committed_errors_on() {
        let input = ByteCursor::new(b"abac");
        assert_eq!(
            optional(committed_ab())
                .parse(ByteCursor::new(b"ac"))
                .unwrap_err()
                .position(),
            1
        );
        assert_eq!(many(committed_ab()).parse(input).unwrap_err().position(), 3);
        assert_eq!(some(committed_ab()).parse(input).unwrap_err().position(), 3);
        assert_eq!(
            skip_many(committed_ab())
                .parse(input)
                .unwrap_err()
                .position(),
            3
        );
        let list = separated_list(committed_ab(), is_byte(b','));
        assert_eq!(
            list.parse(ByteCursor::new(b"ab,ac"))
                .unwrap_err()
                .progress()
                .position(),
            4
        );

        // Without a commit too, an element that matched part of its input
        // fails the repetition; one that matches nothing ends it
        let ab = || is_byte(b'a').and(is_byte(b'b'));
        assert_eq!(
            many(ab()).parse(input).unwrap_err().progress().position(),
            3
        );
        let (values, cursor) = many(ab()).parse(ByteCursor::new(b"abx")).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(cursor.position(), 2);
    }

    #[test]
    fn test_label_keeps_committed_errors() {
        let parser = is_byte(b'(').and(is_byte(b')').commit()).label("unit");
        let message = parser
            .parse(ByteCursor::new(b"(x"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("expected ')'"), "got {message}");
    }

    #[test]
    fn test_not_passes_a_committed_failure_on() {
        let error = not(committed_ab())
            .parse(ByteCursor::new(b"ac"))
            .unwrap_err();
        assert!(error.progress().since(0) == crate::Advance::Committed);
    }

    #[test]
    fn test_commit_survives_wrap_and_recursion() {
        // value = digit | '[' commit(value (',' value)* ']')
        fn value<'code>()
        -> impl Parser<Cursor = ByteCursor<'code>, Output = (), Error = ParsicombError<'code>>
        {
            let list = is_byte(b'[')
                .and(
                    separated_list(lazy(|| value().boxed()), is_byte(b','))
                        .and(is_byte(b']'))
                        .commit(),
                )
                .map(|_| ());
            crate::ascii::digit()
                .map(|_| ())
                .or(list)
                .map_err(ParsicombError::wrap)
        }

        let input = b"[[1,2],[3,x]]";
        let error = many(value()).parse(ByteCursor::new(input)).unwrap_err();
        assert!(error.progress().since(0) == crate::Advance::Committed);
        assert_eq!(error.position(), 10);
        assert!(
            error.to_string().contains("expected a digit, found 'x'"),
            "got {error}"
        );
    }
}
