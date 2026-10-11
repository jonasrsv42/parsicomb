//! Skip something (usually whitespace) before a parser without it counting as
//! a match

use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::error::{ErrorLeaf, ErrorNode, ErrorRank, ParsicombError};
use crate::parser::Parser;
use std::fmt;

/// Error type for [`SkipThen`]
#[derive(Debug)]
pub enum SkipThenError<SkipErr, PErr> {
    /// The skipped parser failed
    Skip(SkipErr),
    /// The parser failed after matching part of its input, or with a real
    /// problem
    Parser(PErr),
    /// The parser matched nothing after the skipped input; reported as no
    /// match at `start`, where the skipping began
    NothingAfterSkip {
        /// The parser's error, at the end of the skipped input
        error: PErr,
        /// Where the skipping began
        start: usize,
    },
}

impl<SkipErr, PErr> fmt::Display for SkipThenError<SkipErr, PErr>
where
    SkipErr: ErrorNode,
    PErr: ErrorNode<Element = SkipErr::Element>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.likely_error())
    }
}

impl<SkipErr, PErr> std::error::Error for SkipThenError<SkipErr, PErr>
where
    SkipErr: ErrorNode,
    PErr: ErrorNode<Element = SkipErr::Element>,
{
}

impl<SkipErr, PErr> ErrorNode for SkipThenError<SkipErr, PErr>
where
    SkipErr: ErrorNode,
    PErr: ErrorNode<Element = SkipErr::Element>,
{
    type Element = SkipErr::Element;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        match self {
            SkipThenError::Skip(error) => error.likely_error(),
            SkipThenError::Parser(error) | SkipThenError::NothingAfterSkip { error, .. } => {
                error.likely_error()
            }
        }
    }

    fn rank(&self) -> ErrorRank {
        match self {
            SkipThenError::Skip(error) => error.rank(),
            SkipThenError::Parser(error) => error.rank(),
            SkipThenError::NothingAfterSkip { error, start } => ErrorRank {
                position: *start,
                ..error.rank()
            },
        }
    }
}

/// Converts to the error of the part that failed; `NothingAfterSkip` is
/// wrapped (one allocation) to keep reporting no match at its start
impl<'code, SkipErr, PErr, T> From<SkipThenError<SkipErr, PErr>> for ParsicombError<'code, T>
where
    SkipErr: ErrorNode<Element = T> + Into<ParsicombError<'code, T>> + Send + Sync + 'code,
    PErr: ErrorNode<Element = T> + Into<ParsicombError<'code, T>> + Send + Sync + 'code,
    T: Atomic,
{
    fn from(error: SkipThenError<SkipErr, PErr>) -> Self {
        match error {
            SkipThenError::Skip(error) => error.into(),
            SkipThenError::Parser(error) => error.into(),
            error @ SkipThenError::NothingAfterSkip { .. } => ParsicombError::wrap(error),
        }
    }
}

/// Parser combinator that skips something (usually whitespace), then parses,
/// without the skipped input counting as a match
///
/// `optional` and repetitions stop only when their parser matches nothing;
/// once it has matched part of its input, a failure is passed on. A plain
/// `whitespace.and(token)` that finds whitespace but no token has matched
/// the whitespace, so `many(whitespace.and(token))` would fail at the end of
/// the list instead of stopping. `skip_then(whitespace, token)` reports that
/// case as no match where the whitespace began, and otherwise behaves like
/// `and`: the error still points at the missing token.
///
/// Skipping whitespace after each token instead (`token.and(whitespace)`)
/// avoids the issue altogether; use `skip_then` in grammars that skip before.
///
/// ```
/// use parsicomb::{ByteCursor, Cursor, Parser, many, skip_then, skip_while};
/// use parsicomb::ascii::u64;
///
/// let spaced_number = skip_then(skip_while(|b: u8| b == b' '), u64());
/// let (numbers, rest) = many(spaced_number).parse(ByteCursor::new(b"1 2 3 ;")).unwrap();
/// assert_eq!(numbers, vec![1, 2, 3]);
/// assert_eq!(rest.position(), 5);
/// ```
pub struct SkipThen<PSkip, P> {
    skip: PSkip,
    parser: P,
}

impl<PSkip, P> SkipThen<PSkip, P> {
    /// Same as [`skip_then`]
    pub fn new(skip: PSkip, parser: P) -> Self {
        SkipThen { skip, parser }
    }
}

impl<PSkip, P> Parser for SkipThen<PSkip, P>
where
    PSkip: Parser,
    P: Parser<Cursor = PSkip::Cursor>,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = SkipThenError<PSkip::Error, P::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (_, after_skip) = self.skip.parse(cursor).map_err(SkipThenError::Skip)?;
        self.parser.parse(after_skip).map_err(|error| {
            if after_skip.position() > cursor.position()
                && error.rank().is_no_match_at(after_skip.position())
            {
                SkipThenError::NothingAfterSkip {
                    error,
                    start: cursor.position(),
                }
            } else {
                SkipThenError::Parser(error)
            }
        })
    }
}

/// Create a parser that skips `skip`, then parses `parser`, without the
/// skipped input counting as a match; see [`SkipThen`]
pub fn skip_then<PSkip, P>(skip: PSkip, parser: P) -> SkipThen<PSkip, P>
where
    PSkip: Parser,
    P: Parser<Cursor = PSkip::Cursor>,
{
    SkipThen::new(skip, parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::and::AndExt;
    use crate::ascii::u64;
    use crate::byte::is_byte;
    use crate::{ByteCursor, many, optional, skip_while};

    fn spaces<'code>()
    -> impl Parser<Cursor = ByteCursor<'code>, Output = (), Error = ParsicombError<'code>> {
        skip_while(|b: u8| b == b' ')
    }

    #[test]
    fn test_nothing_after_the_skip_is_no_match_at_the_start() {
        let parser = optional(skip_then(spaces(), is_byte(b'x')));
        let (value, cursor) = parser.parse(ByteCursor::new(b"   y")).unwrap();
        assert_eq!(value, None);
        assert_eq!(cursor.position(), 0);

        // With `and`, the spaces count as a match and the failure is passed on
        let plain = optional(spaces().and(is_byte(b'x')));
        assert!(plain.parse(ByteCursor::new(b"   y")).is_err());
    }

    #[test]
    fn test_error_still_points_at_the_missing_token() {
        let error = skip_then(spaces(), is_byte(b'x'))
            .parse(ByteCursor::new(b"   y"))
            .unwrap_err();
        assert_eq!(error.rank().position, 0);
        assert_eq!(error.likely_error().loc().position(), 3);
        let converted = ParsicombError::from(error);
        assert_eq!(converted.rank().position, 0);
        assert!(converted.to_string().contains("found 'y'"));
    }

    #[test]
    fn test_failure_past_the_token_start_is_passed_on() {
        let pair = skip_then(spaces(), is_byte(b'a').and(is_byte(b'b')));
        let error = many(pair).parse(ByteCursor::new(b" ab ax")).unwrap_err();
        assert_eq!(error.rank().position, 5);
    }

    #[test]
    fn test_real_problems_are_passed_on() {
        let number = skip_then(spaces(), u64());
        let error = many(number)
            .parse(ByteCursor::new(b" 1 99999999999999999999"))
            .unwrap_err();
        assert!(error.to_string().contains("number too large"));
    }
}
