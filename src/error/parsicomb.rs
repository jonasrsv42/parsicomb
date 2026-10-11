use super::{CodeLoc, ErrorLeaf, ErrorNode, Expected};
use crate::atomic::Atomic;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;

/// Built-in error type of the atomic cursors and leaf parsers
#[derive(Debug)]
pub enum ParsicombError<'code, T: Atomic = u8> {
    /// Advanced from the last element onto the end of input
    UnexpectedEndOfFile(CodeLoc<'code, T>),
    /// Tried to advance a cursor that was already at the end of input
    AlreadyAtEndOfFile(CodeLoc<'code, T>),
    /// Tried to read an element at the end of input
    CannotReadValueAtEof(CodeLoc<'code, T>),
    /// Failure described by a free-form message
    SyntaxError {
        message: Cow<'static, str>,
        loc: CodeLoc<'code, T>,
    },
    /// A leaf parser found something other than what it expected
    Expected {
        expected: Expected<T>,
        loc: CodeLoc<'code, T>,
    },
    /// Wrapped error from another parser combinator
    WrappedError {
        inner: Box<dyn ErrorNode<Element = T> + 'code>,
    },
}

impl<'code, T: Atomic> fmt::Display for ParsicombError<'code, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (headline, loc) = match self {
            ParsicombError::UnexpectedEndOfFile(loc) => ("Unexpected end of file", loc),
            ParsicombError::AlreadyAtEndOfFile(loc) => ("Already at end of file", loc),
            ParsicombError::CannotReadValueAtEof(loc) => ("Cannot read value at EOF", loc),
            ParsicombError::SyntaxError { loc, .. } => ("Syntax error", loc),
            ParsicombError::Expected { loc, .. } => ("Syntax error", loc),
            ParsicombError::WrappedError { inner } => {
                return write!(f, "{}", inner.likely_error());
            }
        };

        write!(f, "{} at ", headline)?;
        loc.write_location(f)?;
        match self {
            ParsicombError::SyntaxError { message, .. } => write!(f, ": {}", message)?,
            ParsicombError::Expected { expected, loc } => {
                write!(f, ": ")?;
                expected.write_message(loc, f)?;
            }
            _ => {}
        }
        writeln!(f)?;
        loc.write_context(f)
    }
}

impl<'code, T: Atomic> Error for ParsicombError<'code, T> {}

impl<'code, T: Atomic> ParsicombError<'code, T> {
    /// Wrap an ErrorNode in a ParsicombError
    pub fn wrap(error: impl ErrorNode<Element = T> + 'code) -> Self {
        ParsicombError::WrappedError {
            inner: Box::new(error),
        }
    }

    /// Returns the position where this error occurred
    pub fn position(&self) -> usize {
        self.loc().position()
    }
}

impl<'code, T: Atomic> ErrorLeaf for ParsicombError<'code, T> {
    type Element = T;

    fn loc(&self) -> CodeLoc<'_, Self::Element> {
        match self {
            ParsicombError::UnexpectedEndOfFile(loc)
            | ParsicombError::AlreadyAtEndOfFile(loc)
            | ParsicombError::CannotReadValueAtEof(loc)
            | ParsicombError::SyntaxError { loc, .. }
            | ParsicombError::Expected { loc, .. } => *loc,
            ParsicombError::WrappedError { inner } => inner.likely_error().loc(),
        }
    }
}

impl<'code, T: Atomic> ErrorNode for ParsicombError<'code, T> {
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        match self {
            ParsicombError::WrappedError { inner } => inner.likely_error(),
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codeloc_eos_empty_data() {
        let empty_data = b"";
        let loc = CodeLoc::new(empty_data, 0);
        let error = ParsicombError::AlreadyAtEndOfFile(loc);

        // Should not panic when displaying
        let display_str = format!("{}", error);
        assert!(display_str.contains("Already at end of file"));

        // Should handle position calculation correctly
        assert_eq!(loc.position(), 0);
    }
    #[test]
    fn test_codeloc_eos_single_byte() {
        let data = b"a";
        let loc = CodeLoc::new(data, 1); // Position 1 = past end
        let error = ParsicombError::CannotReadValueAtEof(loc);

        // Should not panic when displaying
        let display_str = format!("{}", error);
        assert!(display_str.contains("Cannot read value at EOF"));

        // Should handle position calculation correctly
        assert_eq!(loc.position(), 1);
    }
    #[test]
    fn test_codeloc_eos_multiline() {
        let data = b"hello\nworld";
        let loc = CodeLoc::new(data, 11); // Position 11 = past end
        let error = ParsicombError::UnexpectedEndOfFile(loc);

        // Should not panic when displaying
        let display_str = format!("{}", error);
        assert!(display_str.contains("Unexpected end of file"));

        // Should handle position calculation correctly
        assert_eq!(loc.position(), 11);
    }
    #[test]
    fn test_codeloc_eos_line_ending() {
        let data = b"hello\n";
        let loc = CodeLoc::new(data, 6); // Position 6 = past end (after newline)
        let error = ParsicombError::SyntaxError {
            message: "test error".into(),
            loc,
        };

        // Should not panic when displaying
        let display_str = format!("{}", error);
        assert!(display_str.contains("test error"));

        // Should handle position calculation correctly
        assert_eq!(loc.position(), 6);
    }
    #[test]
    fn test_eos_display_output() {
        // Test that EOS errors display correctly without bounds issues
        let data = b"hello\nworld";
        let loc = CodeLoc::new(data, 11); // Position 11 = past end
        let error = ParsicombError::UnexpectedEndOfFile(loc);

        let display_str = format!("{}", error);
        println!("EOS Error Display:\n{}", display_str);

        // Verify it contains expected parts
        assert!(display_str.contains("Unexpected end of file"));
        assert!(display_str.contains("line 2"));
        assert!(display_str.contains("world"));
    }
    #[test]
    fn test_eos_after_newline_display() {
        // Test EOS position right after a newline
        let data = b"hello\n";
        let loc = CodeLoc::new(data, 6); // Position 6 = past end (after newline)
        let error = ParsicombError::CannotReadValueAtEof(loc);

        let display_str = format!("{}", error);
        println!("EOS After Newline:\n{}", display_str);

        // Should be on line 2 with offset 0
        assert!(display_str.contains("line 2"));
        assert!(display_str.contains("column 0"));
    }
    #[test]
    fn test_location_reports_line_column_and_offset() {
        let code = "ab\ncé?".as_bytes();
        let error: ParsicombError = ParsicombError::SyntaxError {
            message: "boom".into(),
            loc: CodeLoc::new(code, code.len() - 1),
        };
        // column counts 'é' once; offset counts its two bytes
        let first = error.to_string().lines().next().unwrap().to_string();
        assert_eq!(first, "Syntax error at line 2, column 2 (offset 6): boom");
    }
    #[test]
    fn test_location_far_offset_falls_back_to_offset_only() {
        let code = vec![b'x'; crate::error::render::MAX_POSITION_SCAN + 8];
        let error: ParsicombError =
            ParsicombError::UnexpectedEndOfFile(CodeLoc::new(&code, code.len()));
        let first = error.to_string().lines().next().unwrap().to_string();
        assert_eq!(
            first,
            format!("Unexpected end of file at offset {}", code.len())
        );
    }
}
