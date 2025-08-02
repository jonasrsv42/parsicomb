use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::error::{ErrorLeaf, ErrorNode};
use crate::parser::Parser;
use std::fmt;

/// Error type for SeparatedList parser
#[derive(Debug)]
pub enum SeparatedListError<E1, E2> {
    /// Error from the element parser
    Element(E1),
    /// Error from the separator parser (only used internally, not returned)
    Separator(E2),
}

impl<E1: fmt::Display, E2: fmt::Display> fmt::Display for SeparatedListError<E1, E2> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeparatedListError::Element(e) => write!(f, "Element failed: {}", e),
            SeparatedListError::Separator(e) => write!(f, "Separator failed: {}", e),
        }
    }
}

impl<E1, E2> std::error::Error for SeparatedListError<E1, E2>
where
    E1: std::error::Error,
    E2: std::error::Error,
{
}

impl<'code, E1, E2, T: Atomic + 'code> ErrorNode<'code> for SeparatedListError<E1, E2>
where
    E1: ErrorNode<'code, Element = T>,
    E2: ErrorNode<'code, Element = T>,
{
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = T> {
        match self {
            SeparatedListError::Element(e) => e.likely_error(),
            SeparatedListError::Separator(e) => e.likely_error(),
        }
    }
}

/// Parser combinator that matches a list of items separated by a parser
///
/// This combinator parses at least one item, followed by zero or more
/// occurrences of (separator + item). It returns a vector of all items.
///
/// # Examples
/// - `"a,b,c"` with separator `,` → `vec!["a", "b", "c"]`
/// - `"1;2;3"` with separator `;` → `vec![1, 2, 3]`
///
/// # Note
/// - Requires at least one element
/// - Trailing separators cause an error
/// - Does not handle whitespace automatically
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
    P: Parser<'code>,
    P::Cursor: Cursor<'code>,
    <P::Cursor as Cursor<'code>>::Element: Atomic + 'code,
    P::Error: ErrorNode<'code, Element = <P::Cursor as Cursor<'code>>::Element>,
    PS: Parser<'code, Cursor = P::Cursor>,
    PS::Error: ErrorNode<'code, Element = <P::Cursor as Cursor<'code>>::Element>,
{
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = P::Error; // Return element parser error directly

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::new();

        // Parse the first element (required)
        let (first_value, mut cursor) = self.parser.parse(cursor)?;
        results.push(first_value);

        // Parse remaining elements preceded by separator
        loop {
            // Try to parse separator
            let temp_cursor = match self.separator.parse(cursor) {
                Ok((_, new_cursor)) => new_cursor,
                Err(_) => break, // No more separators, we're done
            };

            // Parse the next element (required after separator)
            let (value, next_cursor) = self.parser.parse(temp_cursor)?;
            results.push(value);
            cursor = next_cursor;
        }

        Ok((results, cursor))
    }
}

/// Creates a parser that matches a list of items separated by the given parser
///
/// Constraints:
/// - Both parsers must use the same cursor type
/// - Both parsers must have errors with the same element type
pub fn separated_list<'code, P, PS>(parser: P, separator: PS) -> SeparatedList<P, PS>
where
    P: Parser<'code>,
    PS: Parser<'code, Cursor = P::Cursor>,
{
    SeparatedList::new(parser, separator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::ascii::number::i64;
    use crate::byte::is_byte;
    use crate::or::OrExt;
    use crate::utf8::string::is_string;

    #[test]
    fn test_empty_list_fails() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(i64(), is_byte(b','));

        // Should fail on empty input
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_single_element() {
        let data = b"42";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(i64(), is_byte(b','));

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![42]);
    }

    #[test]
    fn test_multiple_elements() {
        let data = b"1,2,3";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(i64(), is_byte(b','));

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![1, 2, 3]);
    }

    #[test]
    fn test_trailing_separator_causes_error() {
        let data = b"1,2,";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(i64(), is_byte(b','));

        // With strict parsing, trailing comma should cause an error
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_missing_element_after_separator_fails() {
        let data = b"1, 2, "; // Space after last comma but no element
        let cursor = ByteCursor::new(data);
        let parser = separated_list(i64(), is_byte(b','));

        // Should fail because there's no number after the last comma
        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_non_matching_separator() {
        let data = b"1;2;3";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(i64(), is_byte(b','));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![1]); // Only first element
        assert_eq!(cursor.value().unwrap(), b';');
    }

    #[test]
    fn test_string_separator() {
        let data = b"apple::banana::cherry";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(
            is_string("apple")
                .or(is_string("banana"))
                .or(is_string("cherry")),
            is_string("::"),
        );

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].as_ref(), "apple");
        assert_eq!(results[1].as_ref(), "banana");
        assert_eq!(results[2].as_ref(), "cherry");
    }

    #[test]
    fn test_with_remaining_content() {
        let data = b"1,2,3 extra";
        let cursor = ByteCursor::new(data);
        let parser = separated_list(i64(), is_byte(b','));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![1, 2, 3]);
        assert_eq!(cursor.value().unwrap(), b' ');
    }
}
