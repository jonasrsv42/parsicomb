//! Parsers over single elements of any [`Atomic`] type

use crate::cursor::Cursor;
use crate::error::Expected;
use crate::parser::Parser;
use std::fmt::{self, Debug, Display};
use std::marker::PhantomData;

pub mod between;
pub mod is;
pub mod sequence;
pub mod skip_while;

pub use between::{AtomicBetween, atomic_between};
pub use is::{IsAtomic, is_atomic};
pub use sequence::{IsSequence, is_sequence};
pub use skip_while::{SkipWhile, SkipWhile1, skip_while, skip_while1};

/// Trait for atomic elements that can be used in parsing
/// This enables generic error formatting and position calculation
///
/// Elements are plain values (bytes, tokens, ...) that do not borrow, so
/// parsers can describe expected elements with `'static` data.
pub trait Atomic: Copy + PartialEq + Debug + Display + Send + Sync + 'static {
    /// Check if this element represents a newline for this atomic type
    ///
    /// Error messages count lines and show the lines around an error with it.
    fn is_newline(&self) -> bool;

    /// Format a slice of elements for display in error messages
    ///
    /// Defaults to each element's `Display`, separated by spaces.
    fn format_slice(slice: &[Self]) -> String {
        let mut text = String::new();
        for (i, element) in slice.iter().enumerate() {
            if i > 0 {
                text.push(' ');
            }
            text.push_str(&element.to_string());
        }
        text
    }

    /// Describe a single element in error messages (the `X` in "expected X")
    ///
    /// Defaults to the element's `Display`.
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }

    /// How many elements of `code` (from `code[0]`, never empty) make up one
    /// displayed unit, such as the bytes of a UTF-8 character
    ///
    /// Error messages show a whole unit as what was found. Defaults to 1.
    fn unit_len(code: &[Self]) -> usize {
        let _ = code;
        1
    }
}

/// A parser that reads one atomic element from the cursor and advances it
/// This is the generic equivalent of a byte parser
pub struct AtomicParser<Cur> {
    _cursor: PhantomData<Cur>,
}

impl<Cur> AtomicParser<Cur> {
    /// Same as [`atomic`]
    pub fn new() -> Self {
        Self {
            _cursor: PhantomData,
        }
    }
}

impl<Cur: Cursor> Default for AtomicParser<Cur> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Cur: Cursor> Parser for AtomicParser<Cur> {
    type Cursor = Cur;
    type Output = Cur::Element;
    type Error = Cur::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        match cursor.value() {
            Ok(value) => Ok((value, cursor.next())),
            Err(_) => Err(cursor.expected_error(Expected::Label("an element"))),
        }
    }
}

/// Convenience function to create an atomic parser for a specific cursor type
pub fn atomic<Cur>() -> AtomicParser<Cur> {
    AtomicParser::new()
}

impl Atomic for u8 {
    fn is_newline(&self) -> bool {
        *self == b'\n'
    }

    fn format_slice(slice: &[Self]) -> String {
        String::from_utf8_lossy(slice).to_string()
    }

    /// The bytes of the UTF-8 character at the start of `code`, or 1 if no
    /// valid character starts there
    fn unit_len(code: &[Self]) -> usize {
        let window = &code[..code.len().min(4)];
        let valid = match std::str::from_utf8(window) {
            Ok(text) => text,
            Err(error) => std::str::from_utf8(&window[..error.valid_up_to()]).unwrap_or_default(),
        };
        valid.chars().next().map_or(1, char::len_utf8)
    }

    /// `'('` for printable ASCII; `byte 0x0A ('\n')` for other bytes, escaped
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if matches!(self, b'\'' | b'\\') {
            write!(f, "'\\{}'", char::from(*self))
        } else if self.is_ascii_graphic() || *self == b' ' {
            write!(f, "'{}'", char::from(*self))
        } else {
            write!(f, "byte 0x{:02X} ('{}')", self, self.escape_ascii())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilterExt;
    use crate::many::many;
    use crate::{ByteCursor, CodeLoc, Parser, ParsicombError};

    // Test implementation of Atomic for u32
    impl Atomic for u32 {
        fn is_newline(&self) -> bool {
            *self == 10 // ASCII newline as u32
        }
    }

    // Note: We can't implement Display for u32 here due to orphan rules
    // u32 already implements Display in std, so this is not needed anyway

    // Custom U32Cursor for testing
    #[derive(Debug, Copy, Clone)]
    pub enum U32Cursor<'code> {
        Valid { data: &'code [u32], position: usize },
        EndOfFile { data: &'code [u32] },
    }

    impl<'code> U32Cursor<'code> {
        pub fn new(data: &'code [u32]) -> Self {
            if data.is_empty() {
                U32Cursor::EndOfFile { data }
            } else {
                U32Cursor::Valid { data, position: 0 }
            }
        }

        fn source(&self) -> &'code [u32] {
            match self {
                U32Cursor::Valid { data, .. } => data,
                U32Cursor::EndOfFile { data } => data,
            }
        }
    }

    impl<'code> Cursor for U32Cursor<'code> {
        type Element = u32;
        type Error = ParsicombError<'code, u32>;
        type Span = crate::Span<'code, u32>;

        fn value(&self) -> Result<Self::Element, Self::Error> {
            match self {
                U32Cursor::Valid { data, position } => Ok(data[*position]),
                U32Cursor::EndOfFile { data } => {
                    Err(ParsicombError::EndOfInput(CodeLoc::new(data, data.len())))
                }
            }
        }

        fn next(self) -> Self {
            match self {
                U32Cursor::Valid { data, position } => {
                    if position + 1 >= data.len() {
                        U32Cursor::EndOfFile { data }
                    } else {
                        U32Cursor::Valid {
                            data,
                            position: position + 1,
                        }
                    }
                }
                U32Cursor::EndOfFile { data } => U32Cursor::EndOfFile { data },
            }
        }

        fn position(&self) -> usize {
            match self {
                U32Cursor::Valid { position, .. } => *position,
                U32Cursor::EndOfFile { data } => data.len(),
            }
        }

        fn syntax_error(&self, message: std::borrow::Cow<'static, str>) -> Self::Error {
            ParsicombError::SyntaxError {
                message,
                loc: CodeLoc::new(self.source(), self.position()),
            }
        }

        fn expected_error(&self, expected: crate::Expected<u32>) -> Self::Error {
            ParsicombError::Expected {
                expected,
                loc: CodeLoc::new(self.source(), self.position()),
            }
        }

        fn span_to(&self, end: Self) -> Self::Span {
            crate::Span::new(self.source(), self.position(), end.position())
        }
    }

    #[test]
    fn test_atomic_parser_with_byte_cursor() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser: AtomicParser<ByteCursor> = atomic();

        let (byte, _) = parser.parse(cursor).unwrap();
        assert_eq!(byte, b'h');
    }

    #[test]
    fn test_atomic_parser_with_u32_cursor() {
        let data = [1u32, 2u32, 3u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();

        let (value, _) = parser.parse(cursor).unwrap();
        assert_eq!(value, 1u32);
    }

    #[test]
    fn test_many_with_byte_cursor() {
        let data = b"aaabbb";
        let cursor = ByteCursor::new(data);
        let parser: AtomicParser<ByteCursor> = atomic();

        let many_parser = many(parser.filter(|&b| b == b'a', "'a'"));
        let (results, _) = many_parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
    }

    #[test]
    fn test_many_with_u32_cursor() {
        let data = [5u32, 5u32, 5u32, 7u32, 8u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();

        let many_parser = many(parser.filter(|&x| x == 5, "5"));
        let (results, _) = many_parser.parse(cursor).unwrap();
        assert_eq!(results, vec![5u32, 5u32, 5u32]);
    }

    #[test]
    fn test_filter_with_byte_cursor() {
        let data = b"A";
        let cursor = ByteCursor::new(data);
        let parser: AtomicParser<ByteCursor> = atomic();

        let filtered = parser.filter(|&b| b.is_ascii_uppercase(), "uppercase");
        let (result, _) = filtered.parse(cursor).unwrap();
        assert_eq!(result, b'A');
    }

    #[test]
    fn test_filter_with_u32_cursor() {
        let data = [42u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();

        let filtered = parser.filter(|&x| x > 40, "> 40");
        let (result, _) = filtered.parse(cursor).unwrap();
        assert_eq!(result, 42u32);
    }

    #[test]
    fn test_filter_failure_with_u32_cursor() {
        let data = [30u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();

        let filtered = parser.filter(|&x| x > 40, "> 40");
        let result = filtered.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_handling_preserves_generic_info() {
        let data = [99u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();

        let filtered = parser.filter(|&x| x < 50, "value < 50");
        let result = filtered.parse(cursor);

        assert!(result.is_err());
        let error = result.unwrap_err();
        // The error should contain information about u32 elements
        let error_string = error.to_string();
        assert!(error_string.contains("expected value < 50"));
    }
}
