use super::byte_cursor::ByteCursor;
use super::parser::Parser;
use std::fmt;

/// Parser combinator that transforms the error of a parser using a mapping function
pub struct MapErr<P, F> {
    parser: P,
    mapper: F,
}

impl<P, F> MapErr<P, F> {
    pub fn new(parser: P, mapper: F) -> Self {
        MapErr { parser, mapper }
    }
}

impl<P, F> fmt::Debug for MapErr<P, F>
where
    P: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapErr")
            .field("parser", &self.parser)
            .field("mapper", &"<function>")
            .finish()
    }
}

impl<'code, P, F, E1, E2> Parser<'code> for MapErr<P, F>
where
    P: Parser<'code, Error = E1>,
    F: Fn(E1) -> E2,
    E2: std::error::Error,
{
    type Output = P::Output;
    type Error = E2;

    fn parse(
        &self,
        cursor: ByteCursor<'code>,
    ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
        self.parser.parse(cursor).map_err(&self.mapper)
    }
}

/// Extension trait to add .map_err() method support for parsers
pub trait MapErrExt<'code>: Parser<'code> + Sized {
    fn map_err<F, E2>(self, mapper: F) -> MapErr<Self, F>
    where
        F: Fn(Self::Error) -> E2,
        E2: std::error::Error,
    {
        MapErr::new(self, mapper)
    }
}

/// Implement MapErrExt for all parsers
impl<'code, P> MapErrExt<'code> for P where P: Parser<'code> {}

/// Convenience function to create a MapErr parser
pub fn map_err<'code, P, F, E1, E2>(parser: P, mapper: F) -> MapErr<P, F>
where
    P: Parser<'code, Error = E1>,
    F: Fn(E1) -> E2,
    E2: std::error::Error,
{
    MapErr::new(parser, mapper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParsicombError;
    use crate::byte_cursor::ByteCursor;

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

    // Simple test parser that always fails with ParsicombError
    struct AlwaysFailParser;

    impl<'code> Parser<'code> for AlwaysFailParser {
        type Output = char;
        type Error = ParsicombError<'code>;

        fn parse(
            &self,
            cursor: ByteCursor<'code>,
        ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
            let (data, position) = cursor.inner();
            Err(ParsicombError::SyntaxError {
                message: "always fails".into(),
                loc: crate::CodeLoc::new(data, position),
            })
        }
    }

    // Simple test parser that always succeeds
    struct AlwaysSucceedParser;

    impl<'code> Parser<'code> for AlwaysSucceedParser {
        type Output = char;
        type Error = ParsicombError<'code>;

        fn parse(
            &self,
            cursor: ByteCursor<'code>,
        ) -> Result<(Self::Output, ByteCursor<'code>), Self::Error> {
            Ok(('x', cursor))
        }
    }

    #[test]
    fn test_map_err_transforms_error_on_failure() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        let parser = AlwaysFailParser.map_err(|_| CustomError::Simple("mapped error".to_string()));
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

        let parser = AlwaysSucceedParser
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

        let parser = AlwaysFailParser.map_err(|_| CustomError::WithCode(404));
        let result = parser.parse(cursor);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CustomError::WithCode(404));
    }

    #[test]
    fn test_map_err_chain() {
        let data = b"test";
        let cursor = ByteCursor::new(data);

        let parser = AlwaysFailParser
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

        let parser = AlwaysFailParser
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
        let parser = AlwaysFailParser.map_err(|_| CustomError::Simple("string error".to_string()));
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
        let parser = map_err(AlwaysFailParser, |_| CustomError::WithCode(42));
        let result = parser.parse(cursor);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CustomError::WithCode(42));
    }
}
