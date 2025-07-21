use crate::and::AndExt;
use crate::byte_cursor::ByteCursor;
use crate::many::many;
use crate::parser::Parser;
use crate::utf8::string::is_string;
use crate::utf8::unicode_whitespace;
use areamy::error::Error;

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

impl<'a, P> Parser<'a> for SeparatedList<P>
where
    P: Parser<'a>,
{
    type Output = Vec<P::Output>;

    fn parse(&self, cursor: ByteCursor<'a>) -> Result<(Self::Output, ByteCursor<'a>), Error> {
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
pub fn separated_list<'a, P>(
    parser: P,
    separator: &'static str,
) -> impl Parser<'a, Output = Vec<P::Output>>
where
    P: Parser<'a>,
{
    SeparatedList::new(parser, separator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapExt;
    use crate::some::some;
    use crate::utf8::alphanumeric::unicode_alphanumeric;

    #[test]
    fn test_empty_list_fails() {
        let data = b"";
        let cursor = ByteCursor::new(data).unwrap();
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
        let cursor = ByteCursor::new(data).unwrap();
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
        let cursor = ByteCursor::new(data).unwrap();
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
        let cursor = ByteCursor::new(data).unwrap();
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
        let cursor = ByteCursor::new(data).unwrap();
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
        let cursor = ByteCursor::new(data).unwrap();
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
        let cursor = ByteCursor::new(data).unwrap();
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
        let cursor = ByteCursor::new(data).unwrap();
        let parser = separated_list(some(unicode_alphanumeric()), ",");

        // Should fail because there's no identifier after the last comma
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_invalid_element_after_separator_fails() {
        let data = b"a, b, |"; // Pipe instead of alphanumeric
        let cursor = ByteCursor::new(data).unwrap();
        let parser = separated_list(some(unicode_alphanumeric()), ",");

        // Should fail because 123 is not a valid identifier
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_non_matching_separator() {
        let data = b"a;b;c";
        let cursor = ByteCursor::new(data).unwrap();
        let parser = separated_list(
            some(unicode_alphanumeric()).map(|chrs| chrs.iter().collect::<String>()),
            ",",
        );

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec!["a"]);
        assert_eq!(cursor.value().unwrap(), b';');
    }
}

