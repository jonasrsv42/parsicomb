//! Deferred parser construction for recursive grammars

use crate::parser::Parser;
use std::cell::OnceCell;

/// A lazy parser that defers the construction of the actual parser until parse time.
/// This is useful for breaking mutual recursion between parsers.
///
/// The parser is built on the first `parse` call and cached for the lifetime of
/// this `Lazy`, so repeated parses (e.g. inside `many`) don't rebuild it. For a
/// recursive grammar each nesting level holds its own `Lazy`, so the cached tree
/// only grows as deep as the deepest input seen.
///
/// A parser that refers to itself through `lazy` must erase its parser type
/// at that point with `.boxed()`, and must have a concrete error type, since
/// both would otherwise contain themselves:
///
/// ```
/// use parsicomb::{BoxedExt, ByteCursor, Parser, ParsicombError, lazy};
/// use parsicomb::and::AndExt;
/// use parsicomb::byte::is_byte;
/// use parsicomb::map::MapExt;
/// use parsicomb::map_err::MapErrExt;
/// use parsicomb::or::OrExt;
///
/// /// Nesting depth of balanced parentheses around an `x`, e.g. `((x))` → 2
/// fn nested<'code>()
/// -> impl Parser<Cursor = ByteCursor<'code>, Output = usize, Error = ParsicombError<'code>> {
///     is_byte(b'x')
///         .map(|_| 0)
///         .or(is_byte(b'(')
///             .and(lazy(|| nested().boxed()))
///             .and(is_byte(b')'))
///             .map(|((_, depth), _)| depth + 1))
///         .map_err(ParsicombError::from)
/// }
///
/// let (depth, _) = nested().parse(ByteCursor::new(b"(((x)))")).unwrap();
/// assert_eq!(depth, 3);
/// ```
pub struct Lazy<F, P> {
    factory: F,
    parser: OnceCell<P>,
}

impl<F, P> Lazy<F, P>
where
    F: Fn() -> P,
    P: Parser,
{
    /// Create a new lazy parser with the given factory function
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            parser: OnceCell::new(),
        }
    }
}

impl<F, P> Parser for Lazy<F, P>
where
    F: Fn() -> P,
    P: Parser,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = P::Error;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        self.parser.get_or_init(&self.factory).parse(cursor)
    }
}

/// Create a lazy parser from a factory function
pub fn lazy<F, P>(factory: F) -> Lazy<F, P>
where
    F: Fn() -> P,
    P: Parser,
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
