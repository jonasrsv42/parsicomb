use super::parser::Parser;
use crate::Cursor;

/// Parser that always succeeds without consuming input and returns the default value of T
pub struct DefaultParser<T, C> {
    default: T,
    _phantom_cursor: std::marker::PhantomData<C>,
}

impl<T, C> DefaultParser<T, C> {
    pub fn new(default: T) -> Self {
        DefaultParser {
            default,
            _phantom_cursor: std::marker::PhantomData,
        }
    }
}

impl<T: Clone, C: Cursor> Parser for DefaultParser<T, C> {
    type Cursor = C;
    type Output = T;
    type Error = C::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        Ok((self.default.clone(), cursor))
    }
}

/// Convenience function to create a default parser
pub fn default<T: Clone, C: Cursor>(default: T) -> DefaultParser<T, C> {
    DefaultParser::new(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ByteCursor, Cursor};

    #[test]
    fn test_default_string() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = default(String::default());

        let (result, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(result, String::default());
        // Should not consume any input
        assert_eq!(remaining.value().unwrap(), b'h');
    }

    #[test]
    fn test_default_i32() {
        let data = b"123";
        let cursor = ByteCursor::new(data);
        let parser = default(i32::default());

        let (result, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(result, 0);
        // Should not consume any input
        assert_eq!(remaining.value().unwrap(), b'1');
    }

    #[test]
    fn test_default_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = default(String::default());

        let (result, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(result, String::default());
        assert!(matches!(remaining, ByteCursor::EndOfFile { .. }));
    }
}
