use super::parser::Parser;
use crate::{Atomic, Cursor, ParsicombError};

/// Parser that always succeeds without consuming input and returns the default value of T
pub struct DefaultParser<T, C> {
    default: T,
    _phantom_cursor: std::marker::PhantomData<C>,
}

impl<'code, T, C> DefaultParser<T, C> {
    pub fn new(default: T) -> Self {
        DefaultParser {
            default,
            _phantom_cursor: std::marker::PhantomData,
        }
    }
}

impl<'code, T, C> Parser<'code> for DefaultParser<T, C>
where
    T: Clone,
    C: Cursor<'code>,
    C::Element: Atomic + 'code,
{
    type Cursor = C;
    type Output = T;
    type Error = ParsicombError<'code, <C as Cursor<'code>>::Element>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        Ok((self.default.clone(), cursor))
    }
}

/// Convenience function to create a default parser
pub fn default<'code, T, C>(default: T) -> DefaultParser<T, C>
where
    T: Clone,
    C: Cursor<'code>,
{
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
