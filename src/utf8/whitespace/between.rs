use super::unicode_whitespace;
use crate::ParsicombError;
use crate::byte_cursor::ByteCursor;
use crate::error::{ErrorLeaf, ErrorNode};
use crate::filter::FilterError;
use crate::many::many;
use crate::parser::Parser;
use std::fmt;

/// Error type for Between parser that can wrap errors from all constituent parsers
#[derive(Debug)]
pub enum BetweenError<'code, E1, E2, E3> {
    /// Error from the opening delimiter parser
    OpenDelimiter(E1),
    /// Error from whitespace after open delimiter
    OpenWhitespace(FilterError<'code, ParsicombError<'code>>),
    /// Error from the content parser
    Content(E2),
    /// Error from whitespace before close delimiter
    CloseWhitespace(FilterError<'code, ParsicombError<'code>>),
    /// Error from the closing delimiter parser
    CloseDelimiter(E3),
}

impl<E1: fmt::Display, E2: fmt::Display, E3: fmt::Display> fmt::Display
    for BetweenError<'_, E1, E2, E3>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BetweenError::OpenDelimiter(e) => write!(f, "Open delimiter failed: {}", e),
            BetweenError::OpenWhitespace(e) => write!(f, "Open whitespace failed: {}", e),
            BetweenError::Content(e) => write!(f, "Content failed: {}", e),
            BetweenError::CloseWhitespace(e) => write!(f, "Close whitespace failed: {}", e),
            BetweenError::CloseDelimiter(e) => write!(f, "Close delimiter failed: {}", e),
        }
    }
}

impl<E1, E2, E3> std::error::Error for BetweenError<'_, E1, E2, E3>
where
    E1: std::error::Error,
    E2: std::error::Error,
    E3: std::error::Error,
{
}

// Implement ErrorBranch for BetweenError to enable furthest-error selection
impl<'code, E1, E2, E3> ErrorNode<'code> for BetweenError<'code, E1, E2, E3>
where
    E1: ErrorNode<'code>,
    E2: ErrorNode<'code>,
    E3: ErrorNode<'code>,
{
    fn likely_error(self) -> Box<dyn ErrorLeaf + 'code> {
        match self {
            BetweenError::OpenDelimiter(e1) => e1.likely_error(),
            BetweenError::OpenWhitespace(e) => e.likely_error(),
            BetweenError::Content(e2) => e2.likely_error(),
            BetweenError::CloseWhitespace(e) => e.likely_error(),
            BetweenError::CloseDelimiter(e3) => e3.likely_error(),
        }
    }
}

/// Parser that matches content between opening and closing delimiters with automatic whitespace handling
///
/// This combinator automatically handles Unicode whitespace around the content.
/// It parses: `open + optional_ws + content + optional_ws + close`
///
/// # Returns
/// Just the `content` value with the delimiters and whitespace discarded.
///
/// # Examples
/// - `"[1.0]"` → `1.0`
/// - `"[ 1.0 ]"` → `1.0`  
/// - `"(hello)"` → `"hello"`
/// - `"{ content }"` → `"content"`
/// Custom Between parser implementation
pub struct Between<P1, P2, P3> {
    open: P1,
    content: P2,
    close: P3,
}

impl<'code, P1, P2, P3> Parser<'code> for Between<P1, P2, P3>
where
    P1: Parser<'code>,
    P2: Parser<'code>,
    P3: Parser<'code>,
{
    type Output = P2::Output;
    type Error = BetweenError<'code, P1::Error, P2::Error, P3::Error>;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
        // Parse: open + whitespace + content + whitespace + close
        let (_, cursor) = self
            .open
            .parse(cursor)
            .map_err(BetweenError::OpenDelimiter)?;
        let (_, cursor) = many(unicode_whitespace())
            .parse(cursor)
            .map_err(|e| BetweenError::OpenWhitespace(e))?;
        let (content_val, cursor) = self.content.parse(cursor).map_err(BetweenError::Content)?;
        let (_, cursor) = many(unicode_whitespace())
            .parse(cursor)
            .map_err(|e| BetweenError::CloseWhitespace(e))?;
        let (_, cursor) = self
            .close
            .parse(cursor)
            .map_err(BetweenError::CloseDelimiter)?;

        Ok((content_val, cursor))
    }
}

pub fn between<'code, P1, P2, P3>(open: P1, content: P2, close: P3) -> Between<P1, P2, P3>
where
    P1: Parser<'code>,
    P2: Parser<'code>,
    P3: Parser<'code>,
{
    Between {
        open,
        content,
        close,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::and::AndExt;
    use crate::ascii::number::f64;
    use crate::byte::is_byte;
    use crate::byte_cursor::ByteCursor;
    use crate::or::OrExt;
    use crate::utf8::string::is_string;
    use crate::utf8::whitespace::separated_pair;

    #[test]
    fn test_brackets_number() {
        let data = b"[42.5]";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - 42.5).abs() < f64::EPSILON);
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_brackets_with_spaces() {
        let data = b"[  3.14  ]";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert!((value - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parentheses_string() {
        let data = b"( hello )";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'('), is_string("hello"), is_byte(b')'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert_eq!(value.as_ref(), "hello");
    }

    #[test]
    fn test_braces() {
        let data = b"{test}";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'{'), is_string("test"), is_byte(b'}'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert_eq!(value.as_ref(), "test");
    }

    #[test]
    fn test_nested_with_separated_pair() {
        // Test the combination we'll use for intervals: [1.0, 2.0]
        let data = b"[1.0, 2.0]";
        let cursor = ByteCursor::new(data);
        let parser = between(
            is_byte(b'['),
            separated_pair(f64(), is_string(","), f64()),
            is_byte(b']'),
        );

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert!((left - 1.0).abs() < f64::EPSILON);
        assert!((right - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_nested_with_extra_whitespace() {
        let data = b"[  1.5  ,  2.5  ]";
        let cursor = ByteCursor::new(data);
        let parser = between(
            is_byte(b'['),
            separated_pair(f64(), is_string(","), f64()),
            is_byte(b']'),
        );

        let ((left, right), _) = parser.parse(cursor).unwrap();
        assert!((left - 1.5).abs() < f64::EPSILON);
        assert!((right - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_unicode_whitespace() {
        // Use various Unicode whitespace characters
        let input = "[\u{2000}42.0\u{3000}]"; // En quad + Ideographic space
        let data = input.as_bytes();
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, _) = parser.parse(cursor).unwrap();
        assert!((value - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_missing_open_delimiter_fails() {
        let data = b"42.0]";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_missing_close_delimiter_fails() {
        let data = b"[42.0";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        assert!(parser.parse(cursor).is_err());
    }

    #[test]
    fn test_with_remaining_content() {
        let data = b"[42.0] extra";
        let cursor = ByteCursor::new(data);
        let parser = between(is_byte(b'['), f64(), is_byte(b']'));

        let (value, cursor) = parser.parse(cursor).unwrap();
        assert!((value - 42.0).abs() < f64::EPSILON);
        assert_eq!(cursor.value().unwrap(), b' ');
    }

    #[test]
    fn test_between_with_or_combinator_and_likely_error_flattening() {
        let data = b"[hello,xyz]";
        let cursor = ByteCursor::new(data);

        // Create a complex nested parser that will create deep error structures:
        // between('[', (("hello" | "hi").and(",").and(("world" | "universe"))), ']')
        // This will fail at "xyz" after successfully parsing "hello,"
        let inner_content = is_string("hello")
            .or(is_string("hi"))
            .and(is_byte(b','))
            .and(is_string("world").or(is_string("universe"))); // Will fail on "xyz"

        let parser = between(is_byte(b'['), inner_content, is_byte(b']'));

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // The error structure should be deeply nested through BetweenError -> AndError chains -> OrError
        let complex_error = result.unwrap_err();

        // Just verify that the error occurred and has some meaningful information
        let error_message = complex_error.to_string();
        assert!(
            error_message.len() > 0,
            "Should have a meaningful error message"
        );

        // The error should indicate content parsing failed since the inner parser failed
        assert!(
            error_message.contains("Content failed"),
            "Should indicate that content parsing failed due to nested and/or failure"
        );
    }

    #[test]
    fn test_complex_nested_combinators_with_likely_error_flattening() {
        let data = b"{start: [hello, badvalue], end: finish}";
        let cursor = ByteCursor::new(data);

        // Create a deeply nested parser structure:
        // between('{', separated_pair(
        //     separated_pair("start", ":", between('[', separated_pair(("hello"|"hi"), ",", ("world"|"universe")), ']')),
        //     ",",
        //     separated_pair("end", ":", ("finish"|"done"))
        // ), '}')
        //
        // This creates a structure like:
        // BetweenError<_, SeparatedPairError<SeparatedPairError<_, BetweenError<_, SeparatedPairError<OrError<...>, OrError<...>, _, _>, _>, _, SeparatedPairError<_, OrError<...>, _, _>>, _>, _>

        let inner_list = separated_pair(
            is_string("hello").or(is_string("hi")), // succeeds
            is_string(","),
            is_string("world").or(is_string("universe")), // fails on "badvalue"
        );

        let bracketed_list = between(is_byte(b'['), inner_list, is_byte(b']'));

        let start_pair = separated_pair(is_string("start"), is_string(":"), bracketed_list);

        let end_pair = separated_pair(
            is_string("end"),
            is_string(":"),
            is_string("finish").or(is_string("done")),
        );

        let main_content = separated_pair(start_pair, is_string(","), end_pair);

        let parser = between(is_byte(b'{'), main_content, is_byte(b'}'));

        let result = parser.parse(cursor);
        assert!(result.is_err());

        // The error structure is extremely deeply nested:
        // BetweenError -> SeparatedPairError -> SeparatedPairError -> BetweenError -> SeparatedPairError -> OrError -> ParsicombError
        let complex_error = result.unwrap_err();

        // This demonstrates the full power of our ErrorBranch recursion system
        let actual_error = complex_error.likely_error();

        // The actual error should be at the position where "badvalue" starts (after "hello, ")
        // Position should be around 15-16 where "badvalue" begins
        let error_pos = actual_error.byte_position();
        assert!(
            error_pos >= 15,
            "actual() should find the error that made it furthest into the input (at 'badvalue'), got position {}",
            error_pos
        );

        // Verify the error message makes sense
        let error_message = actual_error.to_string();
        assert!(
            error_message.len() > 0,
            "Should have a meaningful error message"
        );

        println!("Successfully flattened deeply nested error structure!");
        println!("Furthest error position: {}", error_pos);
        println!("Error message: {}", error_message);
    }
}
