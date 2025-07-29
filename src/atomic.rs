use crate::cursors::Cursor;
use crate::error::ErrorNode;
use crate::parser::Parser;
use std::fmt::{Debug, Display};

/// Trait for atomic elements that can be used in parsing
/// This enables generic error formatting and position calculation
pub trait Atomic: Copy + Clone + PartialEq + Debug + Display {
    /// The newline character/element for this atomic type
    const NEWLINE: Self;

    /// Format a slice of elements for display in error messages
    fn format_slice(slice: &[Self]) -> String;
}

/// A parser that reads one atomic element from the cursor and advances it
/// This is the generic equivalent of a byte parser
pub struct AtomicParser<C> {
    _phantom: std::marker::PhantomData<C>,
}

impl<C> AtomicParser<C> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'code, C> Parser<'code> for AtomicParser<C>
where
    C: Cursor<'code>,
    C::Element: Atomic,
    C::Error: ErrorNode<'code>,
{
    type Cursor = C;
    type Output = C::Element;
    type Error = C::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let value = cursor.value()?;
        let next_cursor = cursor.next();
        Ok((value, next_cursor))
    }
}

/// Convenience function to create an atomic parser for a specific cursor type
pub fn atomic<C>() -> AtomicParser<C> {
    AtomicParser::new()
}

impl Atomic for u8 {
    const NEWLINE: Self = b'\n';

    fn format_slice(slice: &[Self]) -> String {
        String::from_utf8_lossy(slice).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilterExt;
    use crate::many::many;
    use crate::then_optionally::ThenOptionallyExt;
    use crate::{ByteCursor, Parser, ParsicombError, CodeLoc};

    // Test implementation of Atomic for u32
    impl Atomic for u32 {
        const NEWLINE: Self = 10; // ASCII newline as u32
        
        fn format_slice(slice: &[Self]) -> String {
            slice.iter().map(|&x| format!("{}", x)).collect::<Vec<_>>().join(" ")
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
    }

    impl<'code> Cursor<'code> for U32Cursor<'code> {
        type Element = u32;
        type Error = ParsicombError<'code, u32>;

        fn value(&self) -> Result<Self::Element, Self::Error> {
            match self {
                U32Cursor::Valid { data, position } => Ok(data[*position]),
                U32Cursor::EndOfFile { .. } => Err(ParsicombError::CannotReadValueAtEof),
            }
        }

        fn next(self) -> Self {
            match self {
                U32Cursor::Valid { data, position } => {
                    if position + 1 >= data.len() {
                        U32Cursor::EndOfFile { data }
                    } else {
                        U32Cursor::Valid { data, position: position + 1 }
                    }
                }
                U32Cursor::EndOfFile { data } => U32Cursor::EndOfFile { data },
            }
        }

        fn try_next(self) -> Result<Self, Self::Error> {
            match self {
                U32Cursor::Valid { .. } => {
                    let next = self.next();
                    match next {
                        U32Cursor::Valid { .. } => Ok(next),
                        U32Cursor::EndOfFile { data } => Err(ParsicombError::UnexpectedEndOfFile(
                            CodeLoc::new(data, data.len()),
                        )),
                    }
                }
                U32Cursor::EndOfFile { .. } => Err(ParsicombError::AlreadyAtEndOfFile),
            }
        }

        fn position(&self) -> usize {
            match self {
                U32Cursor::Valid { position, .. } => *position,
                U32Cursor::EndOfFile { data } => data.len(),
            }
        }

        fn source(&self) -> &'code [Self::Element] {
            match self {
                U32Cursor::Valid { data, .. } => data,
                U32Cursor::EndOfFile { data } => data,
            }
        }

        fn inner(self) -> (&'code [Self::Element], usize) {
            match self {
                U32Cursor::Valid { data, position } => (data, position),
                U32Cursor::EndOfFile { data } => (data, data.len()),
            }
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
        
        let many_parser = many(parser.filter(|&b| b == b'a', "expected 'a'"));
        let (results, _) = many_parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
    }

    #[test]
    fn test_many_with_u32_cursor() {
        let data = [5u32, 5u32, 5u32, 7u32, 8u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();
        
        let many_parser = many(parser.filter(|&x| x == 5, "expected 5"));
        let (results, _) = many_parser.parse(cursor).unwrap();
        assert_eq!(results, vec![5u32, 5u32, 5u32]);
    }

    #[test]
    fn test_filter_with_byte_cursor() {
        let data = b"A";
        let cursor = ByteCursor::new(data);
        let parser: AtomicParser<ByteCursor> = atomic();
        
        let filtered = parser.filter(|&b| b.is_ascii_uppercase(), "expected uppercase");
        let (result, _) = filtered.parse(cursor).unwrap();
        assert_eq!(result, b'A');
    }

    #[test]
    fn test_filter_with_u32_cursor() {
        let data = [42u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();
        
        let filtered = parser.filter(|&x| x > 40, "expected > 40");
        let (result, _) = filtered.parse(cursor).unwrap();
        assert_eq!(result, 42u32);
    }

    #[test]
    fn test_filter_failure_with_u32_cursor() {
        let data = [30u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();
        
        let filtered = parser.filter(|&x| x > 40, "expected > 40");
        let result = filtered.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_then_optionally_with_byte_cursor() {
        let data = b"AB";
        let cursor = ByteCursor::new(data);
        let parser_a: AtomicParser<ByteCursor> = atomic();
        let parser_b: AtomicParser<ByteCursor> = atomic();
        
        let combined = parser_a
            .filter(|&b| b == b'A', "expected A")
            .then_optionally(parser_b.filter(|&b| b == b'B', "expected B"));
        
        let ((a, b_opt), _) = combined.parse(cursor).unwrap();
        assert_eq!(a, b'A');
        assert_eq!(b_opt, Some(b'B'));
    }

    #[test]
    fn test_then_optionally_with_u32_cursor() {
        let data = [10u32, 20u32];
        let cursor = U32Cursor::new(&data);
        let parser_10: AtomicParser<U32Cursor> = atomic();
        let parser_20: AtomicParser<U32Cursor> = atomic();
        
        let combined = parser_10
            .filter(|&x| x == 10, "expected 10")
            .then_optionally(parser_20.filter(|&x| x == 20, "expected 20"));
        
        let ((first, second_opt), _) = combined.parse(cursor).unwrap();
        assert_eq!(first, 10u32);
        assert_eq!(second_opt, Some(20u32));
    }

    #[test]
    fn test_then_optionally_partial_with_u32_cursor() {
        let data = [10u32, 99u32]; // Second element doesn't match
        let cursor = U32Cursor::new(&data);
        let parser_10: AtomicParser<U32Cursor> = atomic();
        let parser_20: AtomicParser<U32Cursor> = atomic();
        
        let combined = parser_10
            .filter(|&x| x == 10, "expected 10")
            .then_optionally(parser_20.filter(|&x| x == 20, "expected 20"));
        
        let ((first, second_opt), _) = combined.parse(cursor).unwrap();
        assert_eq!(first, 10u32);
        assert_eq!(second_opt, None); // Second parser failed, but that's OK
    }

    #[test]
    fn test_complex_combinator_chain_byte_cursor() {
        let data = b"aaabcd";
        let cursor = ByteCursor::new(data);
        let parser: AtomicParser<ByteCursor> = atomic();
        
        // Parse many 'a's, then optionally a 'b', then filter for specific values
        let complex = many(parser.filter(|&b| b == b'a', "expected 'a'"))
            .then_optionally(
                atomic::<ByteCursor>().filter(|&b| b == b'b', "expected 'b'")
            );
        
        let ((as_vec, b_opt), remaining) = complex.parse(cursor).unwrap();
        assert_eq!(as_vec, vec![b'a', b'a', b'a']);
        assert_eq!(b_opt, Some(b'b'));
        assert_eq!(remaining.value().unwrap(), b'c'); // Should be at 'c'
    }

    #[test]
    fn test_complex_combinator_chain_u32_cursor() {
        let data = [1u32, 1u32, 1u32, 2u32, 3u32, 4u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();
        
        // Parse many 1's, then optionally a 2, then continue
        let complex = many(parser.filter(|&x| x == 1, "expected 1"))
            .then_optionally(
                atomic::<U32Cursor>().filter(|&x| x == 2, "expected 2")
            );
        
        let ((ones_vec, two_opt), remaining) = complex.parse(cursor).unwrap();
        assert_eq!(ones_vec, vec![1u32, 1u32, 1u32]);
        assert_eq!(two_opt, Some(2u32));
        assert_eq!(remaining.value().unwrap(), 3u32); // Should be at 3
    }

    #[test]
    fn test_error_handling_preserves_generic_info() {
        let data = [99u32];
        let cursor = U32Cursor::new(&data);
        let parser: AtomicParser<U32Cursor> = atomic();
        
        let filtered = parser.filter(|&x| x < 50, "expected value < 50");
        let result = filtered.parse(cursor);
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        // The error should contain information about u32 elements
        let error_string = error.to_string();
        assert!(error_string.contains("expected value < 50"));
    }
}
