use crate::cursor::Cursor;
use crate::error::ParsicombError;
use crate::parser::Parser;
use std::marker::PhantomData;

/// A type-erased parser
///
/// Hides the concrete parser and error types behind a trait object, leaving
/// only the cursor and output types visible; errors become
/// [`ParsicombError::WrappedError`]. Use it to:
/// - close a recursive grammar: `lazy(|| expr().boxed())`
/// - cap type growth in very long combinator chains
/// - store parsers of different concrete types in one collection
///
/// It costs one allocation at construction, a virtual call per parse and one
/// allocation per failed parse, so box at a few chosen boundaries.
pub struct BoxedParser<'code, Cur: Cursor, Out> {
    parser: Box<dyn Parser<Cursor = Cur, Output = Out, Error = ParsicombError<'code, Cur::Element>> + 'code>,
}

impl<'code, Cur: Cursor, Out> BoxedParser<'code, Cur, Out> {
    pub fn new<P>(parser: P) -> Self
    where
        P: Parser<Cursor = Cur, Output = Out> + 'code,
    {
        BoxedParser {
            parser: Box::new(WrapError {
                parser,
                _lifetime: PhantomData,
            }),
        }
    }
}

impl<'code, Cur: Cursor, Out> Parser for BoxedParser<'code, Cur, Out> {
    type Cursor = Cur;
    type Output = Out;
    type Error = ParsicombError<'code, Cur::Element>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        self.parser.parse(cursor)
    }
}

/// Wraps the error of the inner parser into a [`ParsicombError`]
struct WrapError<'code, P> {
    parser: P,
    _lifetime: PhantomData<&'code ()>,
}

impl<'code, P> Parser for WrapError<'code, P>
where
    P: Parser,
    P::Error: 'code,
    <P::Cursor as Cursor>::Element: 'code,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = ParsicombError<'code, <P::Cursor as Cursor>::Element>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        self.parser.parse(cursor).map_err(ParsicombError::wrap)
    }
}

/// Extension trait to add .boxed() method support for parsers
pub trait BoxedExt: Parser + Sized {
    fn boxed<'code>(self) -> BoxedParser<'code, Self::Cursor, Self::Output>
    where
        Self: 'code,
    {
        BoxedParser::new(self)
    }
}

/// Implement BoxedExt for all parsers
impl<P: Parser> BoxedExt for P {}
