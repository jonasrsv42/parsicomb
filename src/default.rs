use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use crate::ParsicombError;

/// Parser that always succeeds without consuming input and returns the default value of T
pub struct DefaultParser<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> DefaultParser<T> {
    pub fn new() -> Self {
        DefaultParser {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'code, T> Parser<'code> for DefaultParser<T>
where
    T: Default,
{
    type Output = T;
    type Error = ParsicombError<'code>;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
        Ok((T::default(), cursor))
    }
}

/// Convenience function to create a default parser
pub fn default<T>() -> DefaultParser<T>
where
    T: Default,
{
    DefaultParser::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_string() {
        let data = b"hello";
        let cursor = ByteCursor::new(data);
        let parser = default::<String>();

        let (result, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(result, String::default());
        // Should not consume any input
        assert_eq!(remaining.value().unwrap(), b'h');
    }

    #[test]
    fn test_default_i32() {
        let data = b"123";
        let cursor = ByteCursor::new(data);
        let parser = default::<i32>();

        let (result, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(result, 0);
        // Should not consume any input
        assert_eq!(remaining.value().unwrap(), b'1');
    }

    #[test]
    fn test_default_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = default::<String>();

        let (result, remaining) = parser.parse(cursor).unwrap();
        assert_eq!(result, String::default());
        assert!(matches!(remaining, ByteCursor::EndOfFile { .. }));
    }
}
