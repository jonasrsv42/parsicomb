//! Name what a parser expects, for error messages

use crate::cursor::Cursor;
use crate::error::{ErrorNode, Expected, composite_error};
use crate::parser::Parser;

composite_error! {
    /// Error type for Label parser
    pub enum LabelError<PErr, CurErr> {
        /// The inner parser's error, kept because it got past the start or
        /// describes a real problem
        Inner(PErr),
        /// `expected <label>, found …` at the start
        Labeled(CurErr),
    }
}

/// Parser combinator that describes its inner parser in errors
///
/// When the inner parser fails at its start with an error that only says
/// what was expected there (see [`ErrorLeaf::is_expectation`]), the error
/// becomes `expected <label>, found …`. This turns the error of an `or` chain
/// whose alternatives all fail on the first element into one message naming
/// the whole set. Failures further into the input, and errors that describe
/// a real problem such as an overflowing number, are kept as they are.
///
/// ```
/// use parsicomb::{ByteCursor, LabelExt, OrExt, Parser};
/// use parsicomb::byte::is_byte;
///
/// let sign = is_byte(b'+').or(is_byte(b'-')).label("a sign");
/// let error = sign.parse(ByteCursor::new(b"x")).unwrap_err();
/// assert!(error.to_string().contains("expected a sign, found 'x'"));
/// ```
///
/// [`ErrorLeaf::is_expectation`]: crate::ErrorLeaf::is_expectation
pub struct Label<P> {
    parser: P,
    label: &'static str,
}

impl<P> Label<P> {
    /// Same as [`label`]
    pub fn new(parser: P, label: &'static str) -> Self {
        Label { parser, label }
    }
}

impl<P: Parser> Parser for Label<P> {
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = LabelError<P::Error, <P::Cursor as Cursor>::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        self.parser
            .parse(cursor)
            .map_err(|error| relabel(error, cursor, self.label))
    }
}

/// Replace `error` with `expected <label>` at `cursor` when it only says what
/// was expected there; keep it otherwise
pub(crate) fn relabel<PErr, Cur>(
    error: PErr,
    cursor: Cur,
    label: &'static str,
) -> LabelError<PErr, Cur::Error>
where
    PErr: ErrorNode,
    Cur: Cursor,
{
    let rank = error.rank();
    if rank.is_no_match_at(cursor.position()) {
        LabelError::Labeled(cursor.expected_error(Expected::Label(label)))
    } else {
        LabelError::Inner(error)
    }
}

/// Create a parser that describes `parser` as `label` in errors
pub fn label<P: Parser>(parser: P, label: &'static str) -> Label<P> {
    Label::new(parser, label)
}

/// Extension trait to add .label() method support for parsers
pub trait LabelExt: Parser + Sized {
    /// Describe this parser as `label` in errors, e.g. `"an expression"`
    fn label(self, label: &'static str) -> Label<Self> {
        Label::new(self, label)
    }
}

impl<P: Parser> LabelExt for P {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::and::AndExt;
    use crate::ascii::u64;
    use crate::byte::is_byte;
    use crate::or::OrExt;

    #[test]
    fn test_success_passes_through() {
        let parser = is_byte(b'a').label("an a");
        let (value, cursor) = parser.parse(ByteCursor::new(b"ab")).unwrap();
        assert_eq!(value, b'a');
        assert_eq!(cursor.position(), 1);
    }

    #[test]
    fn test_names_all_alternatives_failing_at_the_start() {
        let parser = is_byte(b'a')
            .or(is_byte(b'b'))
            .or(is_byte(b'c'))
            .label("a, b or c");
        let message = parser.parse(ByteCursor::new(b"x")).unwrap_err().to_string();
        assert!(
            message.contains("expected a, b or c, found 'x'"),
            "got {message}"
        );
    }

    #[test]
    fn test_labels_end_of_input() {
        let parser = is_byte(b'a').label("an a");
        let message = parser.parse(ByteCursor::new(b"")).unwrap_err().to_string();
        assert!(
            message.contains("expected an a, found end of input"),
            "got {message}"
        );
    }

    #[test]
    fn test_keeps_failures_past_the_start() {
        let parser = is_byte(b'a').and(is_byte(b'b')).label("ab");
        let message = parser
            .parse(ByteCursor::new(b"ax"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("expected 'b', found 'x'"), "got {message}");
    }

    #[test]
    fn test_keeps_real_problems_at_the_start() {
        let parser = u64().label("a number");
        let message = parser
            .parse(ByteCursor::new(b"99999999999999999999999"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("number too large"), "got {message}");
    }
}
