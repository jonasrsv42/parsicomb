use crate::atomic::atomic;
use crate::cursor::Cursor;
use crate::error::Expected;
use crate::parser::Parser;
use std::marker::PhantomData;

/// Parser that matches one specific element
pub struct IsAtomic<Cur: Cursor> {
    expected: Cur::Element,
    _cursor: PhantomData<Cur>,
}

impl<Cur: Cursor> Parser for IsAtomic<Cur> {
    type Cursor = Cur;
    type Output = Cur::Element;
    type Error = Cur::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let (value, next) = atomic().parse(cursor)?;
        if value == self.expected {
            Ok((value, next))
        } else {
            Err(cursor.expected_error(Expected::Element(self.expected)))
        }
    }
}

/// Create a parser that matches one specific element
pub fn is_atomic<Cur: Cursor>(expected: Cur::Element) -> IsAtomic<Cur> {
    IsAtomic {
        expected,
        _cursor: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtomicCursor, ByteCursor};

    #[test]
    fn test_matches_expected_element() {
        let parser = is_atomic(b'a');
        let (value, cursor) = parser.parse(ByteCursor::new(b"ab")).unwrap();
        assert_eq!(value, b'a');
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_mismatch_reports_expected_and_found() {
        let parser = is_atomic(b'a');
        let error = parser.parse(ByteCursor::new(b"b")).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("expected byte 0x61 ('a'), found byte 0x62 ('b')"),
            "got {message}"
        );
    }

    #[test]
    fn test_end_of_input_fails() {
        let parser = is_atomic(b'a');
        assert!(parser.parse(ByteCursor::new(b"")).is_err());
    }

    #[test]
    fn test_non_byte_elements_use_default_description() {
        let data = [7u32, 8];
        let parser = is_atomic(8u32);
        let error = parser.parse(AtomicCursor::new(&data)).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("expected 8, found 7"), "got {message}");
        assert_eq!(error.position(), 0);
    }

    #[test]
    fn test_label_describes_found_element() {
        use crate::error::Expected;
        let data = [7u32];
        let cursor = AtomicCursor::new(&data);
        let message = cursor
            .expected_error(Expected::Label("an even number"))
            .to_string();
        assert!(
            message.contains("expected an even number, found 7"),
            "got {message}"
        );
    }
}
