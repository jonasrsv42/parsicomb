use crate::and::AndExt;
use crate::byte_cursor::ByteCursor;
use crate::many::many;
use crate::parser::Parser;
use crate::utf8::string::is_string;
use crate::utf8::unicode_whitespace;

/// Parser combinator that matches a list of items separated by a string separator,
/// with optional whitespace around the separator
pub struct SeparatedList<P> {
    parser: P,
    separator: &'static str,
}

impl<P> SeparatedList<P> {
    pub fn new(parser: P, separator: &'static str) -> Self {
        SeparatedList { parser, separator }
    }
}

impl<'code, P> Parser<'code> for SeparatedList<P>
where
    P: Parser<'code>,
{
    type Output = Vec<P::Output>;
    type Error = P::Error;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
        let mut results = Vec::new();

        // Parse the first element (required)
        let (first_value, mut cursor) = self.parser.parse(cursor)?;
        results.push(first_value);

        // Create separator parser once: whitespace* separator whitespace*
        let separator_parser = many(unicode_whitespace())
            .and(is_string(self.separator))
            .and(many(unicode_whitespace()));

        // Parse remaining elements preceded by separator
        loop {
            match separator_parser.parse(cursor) {
                Ok((_, next_cursor)) => {
                    // Parse the next element (required after separator)
                    let (value, next_cursor) = self.parser.parse(next_cursor)?;
                    results.push(value);
                    cursor = next_cursor;
                }
                Err(_) => {
                    // No separator found, we're done
                    break;
                }
            }
        }

        Ok((results, cursor))
    }
}

/// Creates a parser that matches a list of items separated by the given string,
/// with optional whitespace around the separator
pub fn separated_list<'code, P>(
    parser: P,
    separator: &'static str,
) -> impl Parser<'code, Output = Vec<P::Output>>
where
    P: Parser<'code>,
{
    SeparatedList::new(parser, separator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::and::AndExt;
    use crate::error::ErrorNode;
    use crate::map::MapExt;
    use crate::or::OrExt;
    use crate::some::some;
    use crate::utf8::alphanumeric::unicode_alphanumeric;
    use crate::utf8::string::is_string;

    #[test]
    fn test_empty_list_fails() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        // Should fail on empty input
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_single_element() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec!["hello"]);
    }

    #[test]
    fn test_multiple_elements_no_spaces() {
        let data = b"a,b,c";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_multiple_elements_with_spaces() {
        let data = b"a , b , c";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_multiple_elements_with_newlines() {
        let data = b"a ,\n  b ,\n  c";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_trailing_comma_causes_error() {
        let data = b"a,b,";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        // With strict parsing, trailing comma should cause an error
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_list_without_trailing_comma() {
        let data = b"a,b";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec!["a", "b"]);
        // Should consume everything
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_missing_element_after_separator_fails() {
        let data = b"a, b, "; // Space after last comma but no element
        let cursor = ByteCursor::new(data);
        let parser = separated_list(some(unicode_alphanumeric()), ",");

        // Should fail because there's no identifier after the last comma
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_invalid_element_after_separator_fails() {
        let data = b"a, b, |"; // Pipe instead of alphanumeric
        let cursor = ByteCursor::new(data);
        let parser = separated_list(some(unicode_alphanumeric()), ",");

        // Should fail because 123 is not a valid identifier
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_non_matching_separator() {
        let data = b"a;b;c";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec!["a"]);
        assert_eq!(cursor.value().unwrap(), b';');
    }

    #[test]
    fn test_complex_nested_combinators_with_likely_error_flattening() {
        use crate::byte_cursor::ByteCursor;

        let data = b"hello, world, badvalue";
        let cursor = ByteCursor::new(data);

        // Create a complex nested parser for each element:
        // Each element can be either "hello" | "hi" | "world" | "universe"
        let element_parser = is_string("hello")
            .or(is_string("hi"))
            .or(is_string("world"))
            .or(is_string("universe"));

        let parser = separated_list(element_parser, ",");

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // The error should be from the nested OrError structure when it fails on "badvalue"
        let complex_error = result.unwrap_err();

        // Since separated_list returns P::Error directly, we can test likely_error() on it
        let likely_error = complex_error.likely_error();

        // The likely error should be at the position where "badvalue" starts (after "hello, world, ")
        // Position should be around 14 where "badvalue" begins
        let error_pos = likely_error.byte_position();
        assert!(
            error_pos >= 14,
            "likely_error() should find the error that made it furthest into the input (at 'badvalue'), got position {}",
            error_pos
        );

        // Verify the error message makes sense
        let error_message = likely_error.to_string();
        assert!(
            error_message.len() > 0,
            "Should have a meaningful error message"
        );

        println!("Successfully flattened nested OrError in separated_list!");
        println!("Furthest error position: {}", error_pos);
        println!("Error message: {}", error_message);
    }

    #[test]
    fn test_and_or_mixed_combinators_with_error_handling() {
        use crate::ascii::number::f64;
        use crate::byte::is_byte;
        use crate::byte_cursor::ByteCursor;

        let data = b"1.5, 2.7, bad_number";
        let cursor = ByteCursor::new(data);

        // Create a parser for elements that can be either:
        // - A float followed by an optional 'f' suffix: f64().and(optional('f'))
        // - Or just a plain float: f64()
        let element_parser = f64()
            .and(is_byte(b'f').or(is_byte(b'd'))) // Either 'f' or 'd' suffix, will fail on "bad_number"
            .map(|(num, _)| num)
            .or(f64()); // Fallback to plain float

        let parser = separated_list(element_parser, ",");

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // Test that we can get the likely error from the nested And/Or structure
        let complex_error = result.unwrap_err();
        let likely_error = complex_error.likely_error();

        // The error should be at "bad_number" position (around byte 10+)
        let error_pos = likely_error.byte_position();
        assert!(
            error_pos >= 10,
            "Error should be at 'bad_number' position, got {}",
            error_pos
        );

        println!("Complex And/Or error handling works in separated_list!");
        println!("Error at position: {}", error_pos);
        println!("Error: {}", likely_error.to_string());
    }

    #[test]
    fn test_deeply_nested_combinators_error_propagation() {
        use crate::ascii::number::i64;
        use crate::byte_cursor::ByteCursor;

        let data = b"123, 456, not_a_number, 789";
        let cursor = ByteCursor::new(data);

        // Create a deeply nested parser:
        // ("positive" | "negative" | i64) | ("even" | "odd" | i64) | ("small" | "large" | i64)
        let number_or_word = is_string("positive")
            .or(is_string("negative"))
            .or(i64().map(|_| "number".into()));

        let category_or_word = is_string("even").or(is_string("odd")).or(number_or_word);

        let size_or_category = is_string("small")
            .or(is_string("large"))
            .or(category_or_word);

        let parser = separated_list(size_or_category, ",");

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // Test the deeply nested error flattening
        let complex_error = result.unwrap_err();
        let likely_error = complex_error.likely_error();

        // Should point to "not_a_number" which starts around position 10
        let error_pos = likely_error.byte_position();
        assert!(
            error_pos >= 10,
            "Error should be at 'not_a_number', got position {}",
            error_pos
        );

        println!("Deeply nested Or combinators work with likely_error()!");
        println!("Error position: {}", error_pos);
        println!("Error: {}", likely_error.to_string());
    }
}
