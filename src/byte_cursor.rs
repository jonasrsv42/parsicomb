use crate::cursors::Cursor;
use crate::{CodeLoc, ParsicombError};

#[derive(Debug, Copy, Clone)]
pub enum ByteCursor<'code> {
    /// Cursor pointing at a valid byte position
    Valid {
        data: &'code [u8],
        /// Byte position in the data slice (0-based index)
        position: usize,
    },
    /// Cursor at end of file - no more bytes to read
    EndOfFile { data: &'code [u8] },
}

impl<'code> ByteCursor<'code> {
    pub fn new(data: &'code [u8]) -> Self {
        if data.is_empty() {
            return ByteCursor::EndOfFile { data };
        }
        ByteCursor::Valid { data, position: 0 }
    }
}

impl<'code> Cursor<'code> for ByteCursor<'code> {
    type Element = u8;
    type Error = ParsicombError<'code>;

    fn value(&self) -> Result<Self::Element, Self::Error> {
        match self {
            ByteCursor::Valid { data, position } => Ok(data[*position]),
            ByteCursor::EndOfFile { .. } => Err(ParsicombError::CannotReadValueAtEof),
        }
    }

    fn next(self) -> Self {
        match self {
            ByteCursor::Valid { data, position } => {
                if position + 1 >= data.len() {
                    ByteCursor::EndOfFile { data }
                } else {
                    ByteCursor::Valid {
                        data,
                        position: position + 1,
                    }
                }
            }
            ByteCursor::EndOfFile { data } => ByteCursor::EndOfFile { data },
        }
    }

    fn try_next(self) -> Result<Self, Self::Error> {
        match self {
            ByteCursor::Valid { .. } => {
                let next = self.next();
                match next {
                    ByteCursor::Valid { .. } => Ok(next),
                    ByteCursor::EndOfFile { data } => Err(ParsicombError::UnexpectedEndOfFile(
                        CodeLoc::new(data, data.len()),
                    )),
                }
            }
            ByteCursor::EndOfFile { .. } => Err(ParsicombError::AlreadyAtEndOfFile),
        }
    }

    fn position(&self) -> usize {
        match self {
            ByteCursor::Valid { position, .. } => *position,
            ByteCursor::EndOfFile { data } => data.len(),
        }
    }

    fn source(&self) -> &'code [Self::Element] {
        match self {
            ByteCursor::Valid { data, .. } => data,
            ByteCursor::EndOfFile { data } => data,
        }
    }

    fn inner(self) -> (&'code [Self::Element], usize) {
        match self {
            ByteCursor::Valid { data, position } => (data, position),
            ByteCursor::EndOfFile { data } => (data, data.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let data = b"hello\nworld";
        let cursor = ByteCursor::new(data);

        assert_eq!(cursor.value().unwrap(), b'h');

        let cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'e');
    }

    #[test]
    fn test_newline_handling() {
        let data = b"ab\ncd";
        let mut cursor = ByteCursor::new(data);

        // Move to 'a'
        assert_eq!(cursor.value().unwrap(), b'a');

        // Move to 'b'
        cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'b');

        // Move to '\n'
        cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'\n');

        // Move past '\n' to 'c'
        cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'c');
    }

    #[test]
    fn test_eof() {
        let data = b"ab";
        let mut cursor = ByteCursor::new(data);

        assert_eq!(cursor.value().unwrap(), b'a');
        cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'b');

        // next() returns EndOfFile at EOF
        cursor = cursor.next();
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_edge_case_single_byte() {
        let data = b"x";
        let cursor = ByteCursor::new(data);

        assert_eq!(cursor.value().unwrap(), b'x');

        // Should return EndOfFile when trying to advance past the last byte
        let next = cursor.next();
        assert!(matches!(next, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_empty_data() {
        let data = b"";
        let cursor = ByteCursor::new(data);

        // Empty data should return EOF cursor
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));

        // Trying to read value from EOF should error
        assert!(cursor.value().is_err());
    }

    #[test]
    fn test_null_byte_handling() {
        let data = b"a\0b";
        let mut cursor = ByteCursor::new(data);

        assert_eq!(cursor.value().unwrap(), b'a');

        cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'\0');

        cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'b');

        cursor = cursor.next();
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_consecutive_eof_checks() {
        let data = b"x";
        let cursor = ByteCursor::new(data);

        // First advance should return EOF
        let cursor = cursor.next();
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));

        // EOF cursor should stay at EOF
        let cursor = cursor.next();
        assert!(matches!(cursor, ByteCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_try_next_success() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);

        assert_eq!(cursor.value().unwrap(), b'a');

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), b'b');

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), b'c');
    }

    #[test]
    fn test_try_next_eof_error() {
        let data = b"x";
        let cursor = ByteCursor::new(data);

        // try_next should return error at EOF
        let result = cursor.try_next();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unexpected end of file")
        );
    }

    #[test]
    fn test_copy_independence() {
        let data = b"abcd";
        let cursor = ByteCursor::new(data);

        // Make copies before advancing
        let saved_at_a = cursor;
        let also_at_a = cursor;

        // Advance original cursor
        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), b'b');

        // Saved copies are unaffected
        assert_eq!(saved_at_a.value().unwrap(), b'a');
        assert_eq!(also_at_a.value().unwrap(), b'a');

        // Save another copy at 'b'
        let saved_at_b = cursor;

        // Continue advancing
        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), b'c');

        // All saved positions remain valid
        assert_eq!(saved_at_a.value().unwrap(), b'a');
        assert_eq!(saved_at_b.value().unwrap(), b'b');

        // Can use saved copies to create new paths
        let from_a = saved_at_a.try_next().unwrap();
        assert_eq!(from_a.value().unwrap(), b'b');

        let from_b = saved_at_b.try_next().unwrap();
        assert_eq!(from_b.value().unwrap(), b'c');
    }
}
