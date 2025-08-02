use crate::atomic::Atomic;
use crate::cursor::Cursor;
use crate::{CodeLoc, ParsicombError};

#[derive(Debug, Copy, Clone)]
pub enum AtomicCursor<'code, T: Atomic> {
    Valid { data: &'code [T], position: usize },
    EndOfFile { data: &'code [T] },
}

impl<'code, T: Atomic> AtomicCursor<'code, T> {
    pub fn new(data: &'code [T]) -> Self {
        if data.is_empty() {
            return AtomicCursor::EndOfFile { data };
        }
        AtomicCursor::Valid { data, position: 0 }
    }
}

impl<'code, T: Atomic> Cursor<'code> for AtomicCursor<'code, T> {
    type Element = T;
    type Error = ParsicombError<'code, T>;

    fn value(&self) -> Result<Self::Element, Self::Error> {
        match self {
            AtomicCursor::Valid { data, position } => Ok(data[*position]),
            AtomicCursor::EndOfFile { data } => Err(ParsicombError::CannotReadValueAtEof(
                CodeLoc::new(data, data.len()),
            )),
        }
    }

    fn next(self) -> Self {
        match self {
            AtomicCursor::Valid { data, position } => {
                if position + 1 >= data.len() {
                    AtomicCursor::EndOfFile { data }
                } else {
                    AtomicCursor::Valid {
                        data,
                        position: position + 1,
                    }
                }
            }
            AtomicCursor::EndOfFile { data } => AtomicCursor::EndOfFile { data },
        }
    }

    fn try_next(self) -> Result<Self, Self::Error> {
        match self {
            AtomicCursor::Valid { .. } => {
                let next = self.next();
                match next {
                    AtomicCursor::Valid { .. } => Ok(next),
                    AtomicCursor::EndOfFile { data } => Err(ParsicombError::UnexpectedEndOfFile(
                        CodeLoc::new(data, data.len()),
                    )),
                }
            }
            AtomicCursor::EndOfFile { data } => Err(ParsicombError::AlreadyAtEndOfFile(
                CodeLoc::new(data, data.len()),
            )),
        }
    }

    fn position(&self) -> usize {
        match self {
            AtomicCursor::Valid { position, .. } => *position,
            AtomicCursor::EndOfFile { data } => data.len(),
        }
    }

    fn source(&self) -> &'code [Self::Element] {
        match self {
            AtomicCursor::Valid { data, .. } => data,
            AtomicCursor::EndOfFile { data } => data,
        }
    }

    fn inner(self) -> (&'code [Self::Element], usize) {
        match self {
            AtomicCursor::Valid { data, position } => (data, position),
            AtomicCursor::EndOfFile { data } => (data, data.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations_u8() {
        let data = b"hello\nworld";
        let cursor: AtomicCursor<u8> = AtomicCursor::new(data);

        assert_eq!(cursor.value().unwrap(), b'h');

        let cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'e');
    }

    #[test]
    fn test_eof_u8() {
        let data = b"ab";
        let mut cursor: AtomicCursor<u8> = AtomicCursor::new(data);

        assert_eq!(cursor.value().unwrap(), b'a');
        cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), b'b');

        cursor = cursor.next();
        assert!(matches!(cursor, AtomicCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_empty_data_u8() {
        let data = b"";
        let cursor: AtomicCursor<u8> = AtomicCursor::new(data);

        assert!(matches!(cursor, AtomicCursor::EndOfFile { .. }));
        assert!(cursor.value().is_err());
    }

    #[test]
    fn test_try_next_success_u8() {
        let data = b"abc";
        let cursor: AtomicCursor<u8> = AtomicCursor::new(data);

        assert_eq!(cursor.value().unwrap(), b'a');

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), b'b');

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), b'c');
    }

    #[test]
    fn test_try_next_eof_error_u8() {
        let data = b"x";
        let cursor: AtomicCursor<u8> = AtomicCursor::new(data);

        let result = cursor.try_next();
        assert!(result.is_err());
    }

    #[test]
    fn test_copy_independence_u8() {
        let data = b"abcd";
        let cursor: AtomicCursor<u8> = AtomicCursor::new(data);

        let saved_at_a = cursor;
        let also_at_a = cursor;

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), b'b');

        assert_eq!(saved_at_a.value().unwrap(), b'a');
        assert_eq!(also_at_a.value().unwrap(), b'a');

        let saved_at_b = cursor;

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), b'c');

        assert_eq!(saved_at_a.value().unwrap(), b'a');
        assert_eq!(saved_at_b.value().unwrap(), b'b');

        let from_a = saved_at_a.try_next().unwrap();
        assert_eq!(from_a.value().unwrap(), b'b');

        let from_b = saved_at_b.try_next().unwrap();
        assert_eq!(from_b.value().unwrap(), b'c');
    }

    // Note: Using Atomic implementation for u32 from atomic.rs

    #[test]
    fn test_basic_operations_u32() {
        let data = [1u32, 2, 3, 4, 5];
        let cursor: AtomicCursor<u32> = AtomicCursor::new(&data);

        assert_eq!(cursor.value().unwrap(), 1);

        let cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), 2);

        let cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), 3);
    }

    #[test]
    fn test_eof_u32() {
        let data = [10u32, 20];
        let mut cursor: AtomicCursor<u32> = AtomicCursor::new(&data);

        assert_eq!(cursor.value().unwrap(), 10);
        cursor = cursor.next();
        assert_eq!(cursor.value().unwrap(), 20);

        cursor = cursor.next();
        assert!(matches!(cursor, AtomicCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_empty_data_u32() {
        let data: [u32; 0] = [];
        let cursor: AtomicCursor<u32> = AtomicCursor::new(&data);

        assert!(matches!(cursor, AtomicCursor::EndOfFile { .. }));
        assert!(cursor.value().is_err());
    }

    #[test]
    fn test_try_next_success_u32() {
        let data = [100u32, 200, 300];
        let cursor: AtomicCursor<u32> = AtomicCursor::new(&data);

        assert_eq!(cursor.value().unwrap(), 100);

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), 200);

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), 300);
    }

    #[test]
    fn test_try_next_eof_error_u32() {
        let data = [42u32];
        let cursor: AtomicCursor<u32> = AtomicCursor::new(&data);

        let result = cursor.try_next();
        assert!(result.is_err());
        match result {
            Err(ParsicombError::UnexpectedEndOfFile(_)) => {}
            _ => panic!("Expected UnexpectedEndOfFile error"),
        }
    }

    #[test]
    fn test_copy_independence_u32() {
        let data = [5u32, 10, 15, 20];
        let cursor: AtomicCursor<u32> = AtomicCursor::new(&data);

        let saved_at_5 = cursor;
        let also_at_5 = cursor;

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), 10);

        assert_eq!(saved_at_5.value().unwrap(), 5);
        assert_eq!(also_at_5.value().unwrap(), 5);

        let saved_at_10 = cursor;

        let cursor = cursor.try_next().unwrap();
        assert_eq!(cursor.value().unwrap(), 15);

        assert_eq!(saved_at_5.value().unwrap(), 5);
        assert_eq!(saved_at_10.value().unwrap(), 10);

        let from_5 = saved_at_5.try_next().unwrap();
        assert_eq!(from_5.value().unwrap(), 10);

        let from_10 = saved_at_10.try_next().unwrap();
        assert_eq!(from_10.value().unwrap(), 15);
    }

    #[test]
    fn test_position_and_source_u32() {
        let data = [1u32, 2, 3];
        let cursor: AtomicCursor<u32> = AtomicCursor::new(&data);

        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.source(), &[1, 2, 3]);

        let cursor = cursor.next();
        assert_eq!(cursor.position(), 1);
        assert_eq!(cursor.source(), &[1, 2, 3]);

        let cursor = cursor.next();
        assert_eq!(cursor.position(), 2);

        let cursor = cursor.next();
        assert_eq!(cursor.position(), 3); // At EOF
        assert!(matches!(cursor, AtomicCursor::EndOfFile { .. }));
    }

    #[test]
    fn test_inner_u32() {
        let data = [99u32, 88, 77];
        let cursor: AtomicCursor<u32> = AtomicCursor::new(&data);

        let (source, pos) = cursor.inner();
        assert_eq!(source, &[99, 88, 77]);
        assert_eq!(pos, 0);

        let cursor = cursor.next().next();
        let (source, pos) = cursor.inner();
        assert_eq!(source, &[99, 88, 77]);
        assert_eq!(pos, 2);
    }
}
