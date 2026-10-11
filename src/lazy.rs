use crate::parser::Parser;
use std::cell::OnceCell;
use std::marker::PhantomData;

/// A lazy parser that defers the construction of the actual parser until parse time.
/// This is useful for breaking mutual recursion between parsers.
///
/// The parser is built on the first `parse` call and cached for the lifetime of
/// this `Lazy`, so repeated parses (e.g. inside `many`) don't rebuild it. For a
/// recursive grammar each nesting level holds its own `Lazy`, so the cached tree
/// only grows as deep as the deepest input seen.
pub struct Lazy<'code, F, P>
where
    F: Fn() -> P,
    P: Parser<'code>,
{
    factory: F,
    parser: OnceCell<P>,
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
            parser: OnceCell::new(),
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
        self.parser.get_or_init(&self.factory).parse(cursor)
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

    #[test]
    fn test_factory_called_once() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let parser = many(lazy(|| {
            calls.set(calls.get() + 1);
            is_byte(b'a')
        }));

        let (out, _) = parser.parse(ByteCursor::new(b"aaab")).unwrap();
        assert_eq!(out.len(), 3);
        let _ = parser.parse(ByteCursor::new(b"aa")).unwrap();
        assert_eq!(calls.get(), 1);
    }
}
