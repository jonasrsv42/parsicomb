//! One or more items separated by a separator

use crate::cursor::Cursor;
use crate::error::{Advance, ErrorNode, composite_error};
use crate::parser::Parser;

composite_error! {
    /// Error type for SeparatedList parser
    pub enum SeparatedListError<PErr, PSepErr> {
        /// An item failed
        Item(PErr),
        /// A separator failed after matching part of its input, or inside a
        /// `commit`
        Separator(PSepErr),
    }
}

/// Parser combinator that matches a list of items separated by a parser
///
/// This combinator parses at least one item, followed by zero or more
/// occurrences of (separator + item). It returns a vector of all items.
/// The list ends where the separator fails without advancing
/// ([`Advance::None`]); any other separator failure is passed on.
///
/// # Examples
/// - `"a,b,c"` with separator `,` → `vec!["a", "b", "c"]`
/// - `"1;2;3"` with separator `;` → `vec![1, 2, 3]`
///
/// # Note
/// - Requires at least one element
/// - Trailing separators cause an error
/// - Does not handle whitespace automatically
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct SeparatedList<P, PSep> {
    parser: P,
    separator: PSep,
}

impl<P, PSep> SeparatedList<P, PSep> {
    /// Same as [`separated_list`]
    pub fn new(parser: P, separator: PSep) -> Self {
        SeparatedList { parser, separator }
    }
}

impl<P, PSep> Parser for SeparatedList<P, PSep>
where
    P: Parser,
    PSep: Parser<Cursor = P::Cursor>,
{
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = SeparatedListError<P::Error, PSep::Error>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::new();

        // Parse the first element (required)
        let (first_value, mut cursor) = self
            .parser
            .parse(cursor)
            .map_err(SeparatedListError::Item)?;
        results.push(first_value);

        // Parse remaining elements preceded by separator
        loop {
            let after_separator = match self.separator.parse(cursor) {
                Ok((_, next_cursor)) => next_cursor,
                Err(error) => match error.progress().since(cursor.position()) {
                    Advance::None => break,
                    Advance::Partial | Advance::Committed => {
                        return Err(SeparatedListError::Separator(error));
                    }
                },
            };

            // An element is required after a separator, unless the separator
            // matched nothing (e.g. optional whitespace): then the list ended
            let (value, next_cursor) = match self.parser.parse(after_separator) {
                Ok(result) => result,
                Err(error)
                    if after_separator.position() == cursor.position()
                        && error.progress().since(cursor.position()) == Advance::None =>
                {
                    break;
                }
                Err(error) => return Err(SeparatedListError::Item(error)),
            };

            // Separator + element consumed nothing: stop rather than loop forever
            if next_cursor.position() == cursor.position() {
                break;
            }

            results.push(value);
            cursor = next_cursor;
        }

        Ok((results, cursor))
    }
}

/// Creates a parser that matches a list of items separated by the given parser
pub fn separated_list<P, PSep>(parser: P, separator: PSep) -> SeparatedList<P, PSep>
where
    P: Parser,
    PSep: Parser<Cursor = P::Cursor>,
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

    #[test]
    fn test_non_consuming_separator_and_element_terminates() {
        let cursor = ByteCursor::new(b"b");
        let parser = separated_list(
            crate::optional(is_byte(b'a')),
            crate::optional(is_byte(b',')),
        );

        let (output, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(output, vec![None]);
        assert_eq!(remaining.value().unwrap(), b'b');
    }

    #[test]
    fn test_separator_failures_after_a_match_are_passed_on() {
        use crate::and::AndExt;
        use crate::commit::CommitExt;
        use crate::error::ErrorNode;

        let digit = || crate::ascii::digit();
        // `, ` matched `,` then failed: the input is wrong here
        let list = separated_list(digit(), is_byte(b',').and(is_byte(b' ')));
        let error = list.parse(ByteCursor::new(b"1,x")).unwrap_err();
        assert_eq!(error.progress().position(), 2);
        // Committed separator failures too
        let list = separated_list(digit(), is_byte(b',').and(is_byte(b' ').commit()));
        assert!(
            list.parse(ByteCursor::new(b"1,x"))
                .unwrap_err()
                .progress()
                .since(0)
                == crate::Advance::Committed
        );
        // No separator at all ends the list
        let list = separated_list(digit(), is_byte(b',').and(is_byte(b' ')));
        let (items, cursor) = list.parse(ByteCursor::new(b"1;")).unwrap();
        assert_eq!((items.len(), cursor.position()), (1, 1));
    }
}
