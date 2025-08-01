use crate::byte_cursor::ByteCursor;
use crate::many::many;
use crate::parser::Parser;
use crate::utf8::unicode_whitespace;

/// Parser combinator that matches a list of items separated by a parser,
/// with optional whitespace around the separator
pub struct SeparatedList<P, PS> {
    parser: P,
    separator: PS,
}

impl<P, PS> SeparatedList<P, PS> {
    pub fn new(parser: P, separator: PS) -> Self {
        SeparatedList { parser, separator }
    }
}

impl<'code, P, PS> Parser<'code> for SeparatedList<P, PS>
where
    P: Parser<'code, Cursor = ByteCursor<'code>>,
    PS: Parser<'code, Cursor = ByteCursor<'code>>,
{
    type Cursor = ByteCursor<'code>;
    type Output = Vec<P::Output>;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::new();

        // Parse the first element (required)
        let (first_value, mut cursor) = self.parser.parse(cursor)?;
        results.push(first_value);

        // Parse remaining elements preceded by separator
        loop {
            // Try to parse: whitespace* separator whitespace*
            let (_, temp_cursor) = match many(unicode_whitespace()).parse(cursor) {
                Ok(result) => result,
                Err(_) => break,
            };

            let (_, temp_cursor) = match self.separator.parse(temp_cursor) {
                Ok(result) => result,
                Err(_) => break,
            };

            let (_, temp_cursor) = match many(unicode_whitespace()).parse(temp_cursor) {
                Ok(result) => result,
                Err(_) => break,
            };

            // Parse the next element (required after separator)
            let (value, next_cursor) = self.parser.parse(temp_cursor)?;
            results.push(value);
            cursor = next_cursor;
        }

        Ok((results, cursor))
    }
}

/// Creates a parser that matches a list of items separated by the given parser,
/// with optional whitespace around the separator
pub fn separated_list<'code, P, PS>(parser: P, separator: PS) -> SeparatedList<P, PS>
where
    P: Parser<'code>,
    PS: Parser<'code>,
{
    SeparatedList::new(parser, separator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cursor;
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
            is_string(","),
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
            is_string(","),
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
            is_string(","),
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
            is_string(","),
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
            is_string(","),
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
            is_string(","),
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
            is_string(","),
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
        let parser = separated_list(some(unicode_alphanumeric()), is_string(","));

        // Should fail because there's no identifier after the last comma
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_invalid_element_after_separator_fails() {
        let data = b"a, b, |"; // Pipe instead of alphanumeric
        let cursor = ByteCursor::new(data);
        let parser = separated_list(some(unicode_alphanumeric()), is_string(","));

        // Should fail because 123 is not a valid identifier
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_non_matching_separator() {
        let data = b"a;b;c";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            is_string(","),
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

        let parser = separated_list(element_parser, is_string(","));

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // The error should be from the nested OrError structure when it fails on "badvalue"
        let complex_error = result.unwrap_err();

        // Since separated_list returns P::Error directly, we can test likely_error() on it
        let likely_error = complex_error.likely_error();

        // The likely error should be at the position where "badvalue" starts (after "hello, world, ")
        // Position should be around 14 where "badvalue" begins
        let error_pos = likely_error.loc().position();
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

        let parser = separated_list(element_parser, is_string(","));

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // Test that we can get the likely error from the nested And/Or structure
        let complex_error = result.unwrap_err();
        let likely_error = complex_error.likely_error();

        // The error should be at "bad_number" position (around byte 10+)
        let error_pos = likely_error.loc().position();
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

        let parser = separated_list(size_or_category, is_string(","));

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // Test the deeply nested error flattening
        let complex_error = result.unwrap_err();
        let likely_error = complex_error.likely_error();

        // Should point to "not_a_number" which starts around position 10
        let error_pos = likely_error.loc().position();
        assert!(
            error_pos >= 10,
            "Error should be at 'not_a_number', got position {}",
            error_pos
        );

        println!("Deeply nested Or combinators work with likely_error()!");
        println!("Error position: {}", error_pos);
        println!("Error: {}", likely_error.to_string());
    }

    #[test]
    fn test_byte_separator() {
        use crate::byte::is_byte;

        let data = b"a,b,c";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            is_byte(b','),
        );

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec!["a", "b", "c"]);
    }
}
