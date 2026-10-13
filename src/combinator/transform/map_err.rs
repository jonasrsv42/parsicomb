//! Transform a parser's error

use crate::cursor::Cursor;
use crate::error::ErrorNode;
use crate::parser::Parser;

/// Parser combinator that transforms the error of a parser using a mapping function
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct MapErr<P, F> {
    parser: P,
    mapper: F,
}

impl<P, F> MapErr<P, F> {
    /// Same as [`map_err`]
    pub fn new(parser: P, mapper: F) -> Self {
        MapErr { parser, mapper }
    }
}

impl<P, F, NewErr> Parser for MapErr<P, F>
where
    P: Parser,
    F: Fn(P::Error) -> NewErr,
    NewErr: ErrorNode<Element = <P::Cursor as Cursor>::Element>,
{
    type Cursor = P::Cursor;
    type Output = P::Output;
    type Error = NewErr;

    #[inline]
    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        self.parser.parse(cursor).map_err(&self.mapper)
    }
}

/// Extension trait to add .map_err() method support for parsers
pub trait MapErrExt: Parser + Sized {
    /// Transform the error with `mapper`, e.g. `ParsicombError::from`
    fn map_err<F, NewErr>(self, mapper: F) -> MapErr<Self, F>
    where
        F: Fn(Self::Error) -> NewErr,
        NewErr: ErrorNode<Element = <Self::Cursor as Cursor>::Element>,
    {
        MapErr::new(self, mapper)
    }
}

impl<P: Parser> MapErrExt for P {}

/// Create a parser that transforms `parser`'s error; see [`MapErr`]
pub fn map_err<P, F, NewErr>(parser: P, mapper: F) -> MapErr<P, F>
where
    P: Parser,
    F: Fn(P::Error) -> NewErr,
    NewErr: ErrorNode<Element = <P::Cursor as Cursor>::Element>,
{
    MapErr::new(parser, mapper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::error::Progress;
    use crate::{CodeLoc, ErrorLeaf, ParsicombError};

    use std::fmt;

    // Test error types
    #[derive(Debug, PartialEq)]
    enum CustomError {
        Simple(String),
        WithCode(u32),
    }

    impl fmt::Display for CustomError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                CustomError::Simple(msg) => write!(f, "Simple: {}", msg),
                CustomError::WithCode(code) => write!(f, "WithCode: {}", code),
            }
        }
    }

    impl std::error::Error for CustomError {}

    impl ErrorLeaf for CustomError {
        type Element = u8;

        fn loc(&self) -> CodeLoc<'_, Self::Element> {
            CodeLoc::new(&[], 0) // Default position for test error
        }
    }

    impl ErrorNode for CustomError {
        type Element = u8;

        fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
            self
        }

        fn progress(&self) -> Progress {
            Progress::at(0)
        }
    }

    // Simple test parser that always fails with ParsicombError
    struct AlwaysFailParser<'code>(std::marker::PhantomData<&'code ()>);

    impl<'code> Parser for AlwaysFailParser<'code> {
        type Cursor = ByteCursor<'code>;
        type Output = char;
        type Error = ParsicombError<'code>;

        fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
            let (data, position) = cursor.inner();
            Err(ParsicombError::Invalid {
                message: "always fails".into(),
                loc: crate::CodeLoc::new(data, position),
            })
        }
    }

    // Simple test parser that always succeeds
    struct AlwaysSucceedParser<'code>(std::marker::PhantomData<&'code ()>);

    impl<'code> Parser for AlwaysSucceedParser<'code> {
        type Cursor = ByteCursor<'code>;
        type Output = char;
        type Error = ParsicombError<'code>;

        fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
            Ok(('x', cursor))
        }
    }

    #[test]
    fn test_map_err_transforms_error_on_failure() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        let parser = AlwaysFailParser(std::marker::PhantomData)
            .map_err(|_| CustomError::Simple("mapped error".to_string()));
        let result = parser.parse(cursor);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            CustomError::Simple("mapped error".to_string())
        );
    }

    #[test]
    fn test_map_err_preserves_success() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        let parser = AlwaysSucceedParser(std::marker::PhantomData)
            .map_err(|_| CustomError::Simple("should not be called".to_string()));
        let result = parser.parse(cursor);

        assert!(result.is_ok());
        let (output, _) = result.unwrap();
        assert_eq!(output, 'x');
    }

    #[test]
    fn test_map_err_with_different_error_types() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        let parser =
            AlwaysFailParser(std::marker::PhantomData).map_err(|_| CustomError::WithCode(404));
        let result = parser.parse(cursor);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CustomError::WithCode(404));
    }

    #[test]
    fn test_map_err_chain() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        let parser = AlwaysFailParser(std::marker::PhantomData)
            .map_err(|_| CustomError::Simple("first".to_string()))
            .map_err(|_| CustomError::WithCode(500));

        let result = parser.parse(cursor);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CustomError::WithCode(500));
    }

    #[test]
    fn test_map_err_with_closure_accessing_original_error() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        let parser = AlwaysFailParser(std::marker::PhantomData)
            .map_err(|original_err| CustomError::Simple(format!("Wrapped: {}", original_err)));

        let result = parser.parse(cursor);
        assert!(result.is_err());
        let error_msg = match result.unwrap_err() {
            CustomError::Simple(msg) => msg,
            _ => panic!("Expected Simple error"),
        };
        assert!(error_msg.starts_with("Wrapped:"));
        assert!(error_msg.contains("always fails"));
    }

    #[test]
    fn test_map_err_ext_trait() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        // Test that the extension trait works
        let parser = AlwaysFailParser(std::marker::PhantomData)
            .map_err(|_| CustomError::Simple("string error".to_string()));
        let result = parser.parse(cursor);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            CustomError::Simple("string error".to_string())
        );
    }

    #[test]
    fn test_map_err_convenience_function() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        // Test the standalone function
        let parser = map_err(AlwaysFailParser(std::marker::PhantomData), |_| {
            CustomError::WithCode(42)
        });
        let result = parser.parse(cursor);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CustomError::WithCode(42));
    }
}
