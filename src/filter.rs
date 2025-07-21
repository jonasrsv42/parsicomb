use crate::byte_cursor::ByteCursor;
use crate::parser::Parser;
use crate::{ParsiCombError, CodeLoc};

/// Parser that applies a predicate function to filter the output of another parser
pub struct FilterParser<P, F> {
    parser: P,
    predicate: F,
    error_message: String,
}

impl<P, F> FilterParser<P, F> {
    pub fn new(parser: P, predicate: F, error_message: String) -> Self {
        Self {
            parser,
            predicate,
            error_message,
        }
    }
}

impl<'code, P, F, T> Parser<'code> for FilterParser<P, F>
where
    P: Parser<'code, Output = T>,
    F: Fn(&T) -> bool,
{
    type Output = T;
    
    fn parse(&self, cursor: ByteCursor<'code>) -> Result<(Self::Output, ByteCursor<'code>), ParsiCombError<'code>> {
        let (value, new_cursor) = self.parser.parse(cursor)?;
        
        if (self.predicate)(&value) {
            Ok((value, new_cursor))
        } else {
            let (data, position) = cursor.inner();
            Err(ParsiCombError::SyntaxError {
                message: self.error_message.clone(),
                loc: CodeLoc::new(data, position)
            })
        }
    }
}

/// Extension trait to add filter method to all parsers
pub trait FilterExt<'code>: Parser<'code> {
    fn filter<F>(self, predicate: F, error_message: &str) -> FilterParser<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool,
    {
        FilterParser::new(self, predicate, error_message.to_string())
    }
}

impl<'code, P: Parser<'code>> FilterExt<'code> for P {}

/// Convenience function to create a filtered parser
pub fn filter<'code, P, F>(parser: P, predicate: F, error_message: &str) -> FilterParser<P, F>
where
    P: Parser<'code>,
    F: Fn(&P::Output) -> bool,
{
    FilterParser::new(parser, predicate, error_message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utf8::char::char;

    #[test]
    fn test_filter_success() {
        let input = "a";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data).unwrap();
        
        let parser = char().filter(|c| c.is_alphabetic(), "expected alphabetic character");
        let (result, _) = parser.parse(cursor).unwrap();
        assert_eq!(result, 'a');
    }

    #[test]
    fn test_filter_failure() {
        let input = "1";
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data).unwrap();
        
        let parser = char().filter(|c| c.is_alphabetic(), "expected alphabetic character");
        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected alphabetic character"));
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
            let cursor = ByteCursor::new(data).unwrap();
            
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
            let cursor = ByteCursor::new(data).unwrap();
            
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
            let cursor = ByteCursor::new(data).unwrap();
            
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
            let cursor = ByteCursor::new(data).unwrap();
            
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
        let cursor = ByteCursor::new(data).unwrap();
        
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
        let cursor = ByteCursor::new(data).unwrap();
        
        // Filter for alphabetic AND uppercase - should fail on uppercase check
        let parser = char()
            .filter(|c| c.is_alphabetic(), "expected letter")
            .filter(|c| c.is_uppercase(), "expected uppercase");
            
        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected uppercase"));
    }
}
