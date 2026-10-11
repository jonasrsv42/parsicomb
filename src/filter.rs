use crate::Cursor;
use crate::error::{ErrorLeaf, ErrorNode};
use crate::parser::Parser;
use std::borrow::Cow;
use std::fmt;

/// Error type for filter parser that can wrap either the child parser's error
/// or a filter-specific error
#[derive(Debug)]
pub enum FilterError<E, CursorErr> {
    /// Error from the child parser
    ParserError(E),
    /// Filter predicate failed
    FilterFailed(CursorErr),
}

impl<E: fmt::Display, CursorErr: fmt::Display> fmt::Display for FilterError<E, CursorErr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterError::ParserError(e) => write!(f, "{}", e),
            FilterError::FilterFailed(e) => write!(f, "{}", e),
        }
    }
}

impl<E: std::error::Error, CursorErr: std::error::Error> std::error::Error
    for FilterError<E, CursorErr>
{
}

impl<E, CursorErr> ErrorNode for FilterError<E, CursorErr>
where
    E: ErrorNode,
    CursorErr: ErrorNode<Element = E::Element>,
{
    type Element = E::Element;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        match self {
            FilterError::ParserError(e) => e.likely_error(),
            FilterError::FilterFailed(e) => e.likely_error(),
        }
    }
}

/// Parser that applies a predicate function to filter the output of another parser
pub struct FilterParser<P, F> {
    parser: P,
    predicate: F,
    error_message: Cow<'static, str>,
}

impl<P, F> FilterParser<P, F> {
    pub fn new(parser: P, predicate: F, error_message: Cow<'static, str>) -> Self {
        Self {
            parser,
            predicate,
            error_message,
        }
    }
}

impl<P, F> Parser for FilterParser<P, F>
where
    P: Parser,
    F: Fn(&P::Output) -> bool,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = FilterError<P::Error, <P::Cursor as Cursor>::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (value, new_cursor) = self
            .parser
            .parse(cursor)
            .map_err(FilterError::ParserError)?;

        if (self.predicate)(&value) {
            Ok((value, new_cursor))
        } else {
            Err(FilterError::FilterFailed(
                cursor.syntax_error(self.error_message.clone()),
            ))
        }
    }
}

/// Extension trait to add filter method to all parsers
pub trait FilterExt: Parser {
    fn filter<F>(
        self,
        predicate: F,
        error_message: impl Into<Cow<'static, str>>,
    ) -> FilterParser<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool,
    {
        FilterParser::new(self, predicate, error_message.into())
    }
}

impl<P: Parser> FilterExt for P {}

/// Convenience function to create a filtered parser
pub fn filter<P, F>(
    parser: P,
    predicate: F,
    error_message: impl Into<Cow<'static, str>>,
) -> FilterParser<P, F>
where
    P: Parser,
    F: Fn(&P::Output) -> bool,
{
    FilterParser::new(parser, predicate, error_message.into())
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

        let parser = char().filter(|c| c.is_alphabetic(), "expected alphabetic character");
        let (result, _) = parser.parse(cursor).unwrap();
        assert_eq!(result, 'a');
    }

    #[test]
    fn test_filter_failure() {
        let input = "1";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);

        let parser = char().filter(|c| c.is_alphabetic(), "expected alphabetic character");
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

            let parser = char().filter(|c| c.is_alphabetic(), "expected letter");
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

            let parser = char().filter(|c| c.is_numeric(), "expected digit");
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

            let parser = char().filter(|c| c.is_alphanumeric(), "expected alphanumeric");
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

            let parser = char().filter(|c| c.is_whitespace(), "expected whitespace");
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
            .filter(|c| c.is_alphabetic(), "expected letter")
            .filter(|c| c.is_uppercase(), "expected uppercase");

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
            .filter(|c| c.is_alphabetic(), "expected letter")
            .filter(|c| c.is_uppercase(), "expected uppercase");

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected uppercase")
        );
    }
}
