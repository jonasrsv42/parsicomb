//! Skip something (usually whitespace) before a parser without it counting as
//! a match

use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::error::{Advance, ErrorLeaf, ErrorNode, ParsicombError, Progress};
use crate::parser::Parser;
use std::fmt;

/// Error type for [`SkipThen`]
#[derive(Debug)]
pub enum SkipThenError<SkipErr, PErr> {
    /// The skipped parser failed
    Skip(SkipErr),
    /// The parser failed after making progress, or with nothing skipped
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

    fn progress(&self) -> Progress {
        match self {
            SkipThenError::Skip(error) => error.progress(),
            SkipThenError::Parser(error) => error.progress(),
            // Still reported where it failed, but the skipped input doesn't count
            SkipThenError::NothingAfterSkip { error, start } => {
                error.progress().peeked_from(*start)
            }
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
#[must_use = "parsers do nothing until `.parse()` is called"]
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

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (_, after_skip) = self.skip.parse(cursor).map_err(SkipThenError::Skip)?;
        self.parser.parse(after_skip).map_err(|error| {
            if after_skip.position() > cursor.position()
                && error.progress().since(after_skip.position()) == Advance::None
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
    fn test_nothing_after_the_skip_is_no_match() {
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
        assert_eq!(error.progress().position(), 3);
        assert_eq!(error.progress(), Progress::Peek { from: 0, at: 3 });
        assert_eq!(error.progress().since(0), Advance::None);
        let converted = ParsicombError::from(error);
        assert_eq!(converted.progress(), Progress::Peek { from: 0, at: 3 });
        assert!(converted.to_string().contains("found 'y'"));
    }

    #[test]
    fn test_or_keeps_a_failure_over_a_further_missing_token() {
        // `skip_then`'s failure is at 2 but matched nothing; the other
        // alternative matched a space, so its failure at 1 is what decides
        use crate::map::MapExt;
        use crate::or::OrExt;
        let parser =
            skip_then(spaces(), is_byte(b'x')).or(is_byte(b' ').and(is_byte(b'q')).map(|_| b'q'));
        let error = parser.parse(ByteCursor::new(b"  z")).unwrap_err();
        assert!(
            error.to_string().contains("offset 1): expected 'q'"),
            "got {error}"
        );
    }

    #[test]
    fn test_failure_past_the_token_start_is_passed_on() {
        let pair = skip_then(spaces(), is_byte(b'a').and(is_byte(b'b')));
        let error = many(pair).parse(ByteCursor::new(b" ab ax")).unwrap_err();
        assert_eq!(error.progress().position(), 5);
    }

    #[test]
    fn test_real_problems_are_passed_on() {
        let number = skip_then(spaces(), u64());
        let error = many(number)
            .parse(ByteCursor::new(b" 1 99999999999999999999"))
            .unwrap_err();
        assert!(error.to_string().contains("number too large"));
    }

    #[test]
    fn test_label_keeps_a_failure_after_the_skip_where_it_is() {
        use crate::label::LabelExt;
        let parser = skip_then(spaces(), is_byte(b'x')).label("an x");
        let error = parser.parse(ByteCursor::new(b"   ;")).unwrap_err();
        assert_eq!(error.progress().since(0), Advance::None);
        assert!(
            error
                .to_string()
                .contains("offset 3): expected 'x', found ';'"),
            "got {error}"
        );
    }

    #[test]
    fn test_or_classification_does_not_depend_on_order() {
        use crate::map::MapExt;
        use crate::or::OrExt;
        let skipped = || skip_then(spaces(), is_byte(b'x'));
        let matched = || spaces().and(is_byte(b'y')).map(|(_, y)| y);
        let input = ByteCursor::new(b"  ;");
        let one = skipped().or(matched()).parse(input).unwrap_err();
        let other = matched().or(skipped()).parse(input).unwrap_err();
        // `matched` consumed the spaces, so either way it advanced (`Advance::Partial`)
        assert_eq!(one.progress().since(0), Advance::Partial);
        assert_eq!(other.progress().since(0), Advance::Partial);
    }
}
