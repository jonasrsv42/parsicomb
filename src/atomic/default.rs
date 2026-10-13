//! A parser that succeeds with a value without consuming input

use crate::Cursor;
use crate::parser::Parser;

/// Parser that always succeeds without consuming input, returning a clone of the given value
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct DefaultParser<Out, Cur> {
    default: Out,
    _cursor: std::marker::PhantomData<Cur>,
}

impl<Out, Cur> DefaultParser<Out, Cur> {
    /// Same as [`default`]
    pub fn new(default: Out) -> Self {
        DefaultParser {
            default,
            _cursor: std::marker::PhantomData,
        }
    }
}

impl<Out: Clone, Cur: Cursor> Parser for DefaultParser<Out, Cur> {
    type Cursor = Cur;
    type Output = Out;
    type Error = Cur::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        Ok((self.default.clone(), cursor))
    }
}

/// Create a parser that outputs `default` without consuming input; see
/// [`DefaultParser`]
pub fn default<Out: Clone, Cur: Cursor>(default: Out) -> DefaultParser<Out, Cur> {
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
