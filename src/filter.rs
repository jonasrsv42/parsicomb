//! Reject parser outputs that fail a predicate

use crate::Cursor;
use crate::error::Expected;
use crate::label::{LabelError, relabel};
use crate::parser::Parser;

/// Error type for [`Filter`]: [`LabelError::Labeled`] when the predicate
/// rejects the output or the inner parser fails with an expectation at the
/// start, [`LabelError::Inner`] for other inner errors
pub type FilterError<PErr, CurErr> = LabelError<PErr, CurErr>;

/// Parser combinator whose output must satisfy a predicate, described by a
/// label
///
/// Fails with `expected <label>, found …` at its start when the predicate
/// rejects the output, or when the inner parser fails at its start with an
/// expectation (as [`Label`](crate::label::Label) does). Other inner errors,
/// such as invalid UTF-8 under `char()`, are kept. When filters are chained,
/// the outermost label is the one reported.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct Filter<P, F> {
    parser: P,
    predicate: F,
    label: &'static str,
}

impl<P, F> Filter<P, F> {
    /// Same as [`filter`]
    pub fn new(parser: P, predicate: F, label: &'static str) -> Self {
        Filter {
            parser,
            predicate,
            label,
        }
    }
}

impl<P, F> Parser for Filter<P, F>
where
    P: Parser,
    F: Fn(&P::Output) -> bool,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = FilterError<P::Error, <P::Cursor as Cursor>::Error>;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match self.parser.parse(cursor) {
            Ok((value, next)) if (self.predicate)(&value) => Ok((value, next)),
            Ok(_) => Err(LabelError::Labeled(
                cursor.expected_error(Expected::Label(self.label)),
            )),
            Err(error) => Err(relabel(error, cursor, self.label)),
        }
    }
}

/// Extension trait to add filter method to all parsers
pub trait FilterExt: Parser {
    /// Keep only outputs matching `predicate`; `label` names what was expected,
    /// e.g. `"a letter"` gives `expected a letter, found …`
    fn filter<F>(self, predicate: F, label: &'static str) -> Filter<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool,
    {
        Filter::new(self, predicate, label)
    }
}

impl<P: Parser> FilterExt for P {}

/// Create a parser whose output must satisfy `predicate`, described by
/// `label` in errors
pub fn filter<P, F>(parser: P, predicate: F, label: &'static str) -> Filter<P, F>
where
    P: Parser,
    F: Fn(&P::Output) -> bool,
{
    Filter::new(parser, predicate, label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::utf8::char::char;

    #[test]
    fn test_filter_success() {
        let input = "a";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);

        let parser = char().filter(|c| c.is_alphabetic(), "alphabetic character");
        let (result, _) = parser.parse(cursor).unwrap();
        assert_eq!(result, 'a');
    }

    #[test]
    fn test_filter_failure() {
        let input = "1";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);

        let parser = char().filter(|c| c.is_alphabetic(), "alphabetic character");
        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected alphabetic character")
        );
    }

    #[test]
    fn test_filter_unicode_letter() {
        let test_cases = [
            ("a", true),
            ("Z", true),
            ("ñ", true),
            ("中", true),
            ("1", false),
            ("!", false),
            (" ", false),
        ];

        for (input, should_succeed) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);

            let parser = char().filter(|c| c.is_alphabetic(), "letter");
            let result = parser.parse(cursor);

            if should_succeed {
                assert!(result.is_ok(), "Expected success for: {}", input);
                let (ch, _) = result.unwrap();
                assert_eq!(ch, input.chars().next().unwrap());
            } else {
                assert!(result.is_err(), "Expected failure for: {}", input);
            }
        }
    }

    #[test]
    fn test_filter_unicode_digit() {
        let test_cases = [
            ("0", true),
            ("9", true),
            ("٥", true),  // Arabic-Indic digit
            ("５", true), // Fullwidth digit
            ("a", false),
            ("!", false),
        ];

        for (input, should_succeed) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);

            let parser = char().filter(|c| c.is_numeric(), "digit");
            let result = parser.parse(cursor);

            if should_succeed {
                assert!(result.is_ok(), "Expected success for: {}", input);
            } else {
                assert!(result.is_err(), "Expected failure for: {}", input);
            }
        }
    }

    #[test]
    fn test_filter_unicode_alphanumeric() {
        let test_cases = [
            ("a", true),
            ("Z", true),
            ("5", true),
            ("ñ", true),
            ("中", true),
            ("٥", true),
            ("!", false),
            (" ", false),
            ("@", false),
        ];

        for (input, should_succeed) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);

            let parser = char().filter(|c| c.is_alphanumeric(), "alphanumeric");
            let result = parser.parse(cursor);

            if should_succeed {
                assert!(result.is_ok(), "Expected success for: {}", input);
            } else {
                assert!(result.is_err(), "Expected failure for: {}", input);
            }
        }
    }

    #[test]
    fn test_filter_unicode_whitespace() {
        let test_cases = [
            (" ", true),
            ("\t", true),
            ("\n", true),
            ("\r", true),
            ("\u{00A0}", true), // Non-breaking space
            ("\u{2000}", true), // En quad
            ("a", false),
            ("1", false),
            ("!", false),
        ];

        for (input, should_succeed) in test_cases {
            let data = input.as_bytes();
            let cursor = ByteCursor::new(data);

            let parser = char().filter(|c| c.is_whitespace(), "whitespace");
            let result = parser.parse(cursor);

            if should_succeed {
                assert!(result.is_ok(), "Expected success for: {}", input);
            } else {
                assert!(result.is_err(), "Expected failure for: {}", input);
            }
        }
    }

    #[test]
    fn test_chained_filters() {
        let input = "A";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);

        // Filter for alphabetic AND uppercase
        let parser = char()
            .filter(|c| c.is_alphabetic(), "letter")
            .filter(|c| c.is_uppercase(), "uppercase");

        let (result, _) = parser.parse(cursor).unwrap();
        assert_eq!(result, 'A');
    }

    #[test]
    fn test_chained_filters_failure() {
        let input = "a";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);

        // Filter for alphabetic AND uppercase - should fail on uppercase check
        let parser = char()
            .filter(|c| c.is_alphabetic(), "letter")
            .filter(|c| c.is_uppercase(), "uppercase");

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected uppercase")
        );
    }

    #[test]
    fn test_filter_error_names_label_and_found_character() {
        let parser = char().filter(|c| c.is_alphabetic(), "a letter");

        let error = parser.parse(ByteCursor::new("🚀".as_bytes())).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("expected a letter, found \"🚀\""),
            "got {message}"
        );

        let error = parser.parse(ByteCursor::new(b"")).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("expected a letter, found end of input"),
            "got {message}"
        );
    }

    #[test]
    fn test_filter_keeps_inner_errors_that_describe_a_problem() {
        use crate::ascii::u64;

        let parser = u64().filter(|v| *v < 256, "a byte value");
        let message = parser
            .parse(ByteCursor::new(b"99999999999999999999999"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("number too large"), "got {message}");

        let parser = char().filter(|c| c.is_alphabetic(), "a letter");
        let message = parser
            .parse(ByteCursor::new(b"\xC3("))
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("invalid UTF-8 continuation byte"),
            "got {message}"
        );
    }
}
