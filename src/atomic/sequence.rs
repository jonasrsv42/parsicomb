//! A specific run of elements

use crate::atomic::atomic;
use crate::cursor::Cursor;
use crate::error::Expected;
use crate::parser::Parser;
use std::borrow::Cow;
use std::marker::PhantomData;

/// Parser that matches a specific run of elements
///
/// Outputs the span it matched. Pass a `'static` slice (a literal or a static
/// array) to keep failures allocation-free; an owned sequence is cloned into
/// each error.
pub struct IsSequence<Cur: Cursor> {
    expected: Cow<'static, [Cur::Element]>,
    _cursor: PhantomData<Cur>,
}

impl<Cur: Cursor> Parser for IsSequence<Cur> {
    type Cursor = Cur;
    type Output = Cur::Span;
    type Error = Cur::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut current = cursor;

        for (matched, expected) in self.expected.iter().enumerate() {
            match atomic().parse(current) {
                Ok((found, next)) if found == *expected => current = next,
                _ => {
                    return Err(current.expected_error(Expected::Sequence {
                        expected: self.expected.clone(),
                        matched,
                    }));
                }
            }
        }

        Ok((cursor.span_to(current), current))
    }
}

/// Create a parser that matches a specific run of elements
pub fn is_sequence<Cur: Cursor>(
    expected: impl Into<Cow<'static, [Cur::Element]>>,
) -> IsSequence<Cur> {
    IsSequence {
        expected: expected.into(),
        _cursor: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtomicCursor, ByteCursor};

    #[test]
    fn test_matches_and_outputs_span() {
        let parser = is_sequence(&b"let"[..]);
        let (span, cursor) = parser.parse(ByteCursor::new(b"let x")).unwrap();
        assert_eq!(span.slice(), b"let");
        assert_eq!(cursor.position(), 3);
    }

    #[test]
    fn test_mismatch_points_at_first_differing_element() {
        let parser = is_sequence(&b"hello"[..]);
        let error = parser.parse(ByteCursor::new(b"help")).unwrap_err();
        assert_eq!(error.position(), 3);
        let message = error.to_string();
        assert!(
            message.contains(r#"expected "hello", found "help""#),
            "got {message}"
        );
    }

    #[test]
    fn test_end_of_input() {
        let parser = is_sequence(&b"hello"[..]);
        let message = parser
            .parse(ByteCursor::new(b"he"))
            .unwrap_err()
            .to_string();
        assert!(
            message.contains(r#"expected "hello", found "he" at end of input"#),
            "got {message}"
        );

        let message = parser.parse(ByteCursor::new(b"")).unwrap_err().to_string();
        assert!(message.contains("found end of input"), "got {message}");
    }

    #[test]
    fn test_found_span_completes_multibyte_character() {
        let parser = is_sequence("aé".as_bytes());
        let message = parser
            .parse(ByteCursor::new("a€".as_bytes()))
            .unwrap_err()
            .to_string();
        assert!(
            message.contains(r#"expected "aé", found "a€""#),
            "got {message}"
        );
    }

    #[test]
    fn test_non_byte_sequences() {
        static EXPECTED: [u32; 2] = [1, 2];
        let data = [1u32, 3];
        let parser = is_sequence(&EXPECTED[..]);
        let error = parser.parse(AtomicCursor::new(&data)).unwrap_err();
        assert_eq!(error.position(), 1);
        assert!(error.to_string().contains(r#"expected "1 2", found "1 3""#));
    }

    #[test]
    fn test_found_stops_at_the_end_of_the_character() {
        // `C3 80` is `À`; the stray continuation bytes after it are not part of it
        let data = b"a\xC3\x80\x80\x80\x80";
        let error = is_sequence::<ByteCursor>(&b"ab"[..])
            .parse(ByteCursor::new(data))
            .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains(r#"expected "ab", found "aÀ""#),
            "got {message}"
        );
    }
}
