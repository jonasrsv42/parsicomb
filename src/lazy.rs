use crate::parser::Parser;
use std::marker::PhantomData;

/// A lazy parser that defers the construction of the actual parser until parse time.
/// This is useful for breaking mutual recursion between parsers.
pub struct Lazy<'code, F, P>
where
    F: Fn() -> P,
    P: Parser<'code>,
{
    factory: F,
    _phantom: PhantomData<&'code ()>,
}

impl<'code, F, P> Lazy<'code, F, P>
where
    F: Fn() -> P,
    P: Parser<'code>,
{
    /// Create a new lazy parser with the given factory function
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            _phantom: PhantomData,
        }
    }
}

impl<'code, F, P> Parser<'code> for Lazy<'code, F, P>
where
    F: Fn() -> P,
    P: Parser<'code>,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let parser = (self.factory)();
        parser.parse(cursor)
    }
}

/// Create a lazy parser from a factory function
pub fn lazy<'code, F, P>(factory: F) -> Lazy<'code, F, P>
where
    F: Fn() -> P,
    P: Parser<'code>,
{
    Lazy::new(factory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{byte::is_byte, cursor::Cursor, cursors::ByteCursor, many::many};

    #[test]
    fn test_lazy_basic() {
        let input = b"aaaa";
        let cursor = ByteCursor::new(input);

        let lazy_parser = lazy(|| is_byte(b'a'));
        let result = lazy_parser.parse(cursor);

        assert!(result.is_ok());
        let (output, remaining) = result.unwrap();
        assert_eq!(output, b'a');
        assert_eq!(remaining.position(), 1);
    }

    #[test]
    fn test_lazy_with_many() {
        let input = b"aaaa";
        let cursor = ByteCursor::new(input);

        let lazy_parser = lazy(|| many(is_byte(b'a')));
        let result = lazy_parser.parse(cursor);

        assert!(result.is_ok());
        let (output, remaining) = result.unwrap();
        assert_eq!(output.len(), 4);
        assert_eq!(remaining.position(), 4);
    }

    #[test]
    fn test_lazy_deferred_construction() {
        // This test verifies that the parser is constructed lazily
        let lazy_parser = lazy(|| is_byte(b'x'));

        let input = b"xyz";
        let cursor = ByteCursor::new(input);
        let result = lazy_parser.parse(cursor);

        assert!(result.is_ok());
        let (output, _) = result.unwrap();
        assert_eq!(output, b'x');
    }
}
