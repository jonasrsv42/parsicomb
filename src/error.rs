use crate::atomic::Atomic;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;

/// Trait for errors that can report their location in the input
/// This enables selecting the error that progressed furthest when multiple parsers fail
pub trait ErrorLeaf: Error {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// Returns the location where this error occurred
    fn loc(&self) -> CodeLoc<'_, Self::Element>;
}

/// Generic trait for error types that can be flattened to find the furthest error
///
/// This trait enables automatic furthest-error selection across all combinator types
/// (Or, And, Filter, etc.) by providing a way to flatten nested error structures
/// and find the error that made it furthest into the input.
///
/// # Example for downstream crates
///
/// ```rust
/// use parsicomb::error::{ErrorLeaf, ErrorNode, CodeLoc};
/// use std::error::Error;
/// use std::fmt;
///
/// // Your custom error type, borrowing the input it points into
/// #[derive(Debug)]
/// struct MyError<'code> {
///     code: &'code [u8],
///     position: usize,
///     message: String,
/// }
///
/// impl<'code> fmt::Display for MyError<'code> {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", self.message)
///     }
/// }
///
/// impl<'code> Error for MyError<'code> {}
///
/// impl<'code> ErrorLeaf for MyError<'code> {
///     type Element = u8;
///
///     fn loc(&self) -> CodeLoc<'_, Self::Element> {
///         CodeLoc::new(self.code, self.position)
///     }
/// }
///
/// // A leaf error is its own most likely error
/// impl<'code> ErrorNode for MyError<'code> {
///     type Element = u8;
///
///     fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
///         self
///     }
/// }
/// ```
pub trait ErrorNode: Error {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// Flatten nested error structures and return the likely error that made it furthest
    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element>;
}

pub use crate::error_fmt::ReadablePosition;

#[derive(Debug, Copy, Clone)]
pub struct CodeLoc<'code, T: Atomic = u8> {
    code: &'code [T],
    /// The position in `code` where the cursor encountered an error
    loc: usize,
}

impl<'code, T: Atomic> CodeLoc<'code, T> {
    pub fn new(code: &'code [T], loc: usize) -> Self {
        Self { code, loc }
    }

    pub fn position(&self) -> usize {
        self.loc
    }

    /// The input this location points into
    pub fn code(&self) -> &'code [T] {
        self.code
    }
}

impl<'code, T: Atomic> CodeLoc<'code, T> {
    /// Calculate line number and character offset within that line, or `None`
    /// if the error position is too far into the input to scan cheaply.
    ///
    /// Delegates to [`crate::error_fmt`]; see there for details.
    fn readable_position(&self) -> Option<ReadablePosition> {
        crate::error_fmt::readable_position(self.code, self.loc)
    }

    /// Get lines of context around the error position.
    ///
    /// Returns up to 2 lines before and after the error line. Only a bounded
    /// window around the error is materialized, so this stays cheap even when
    /// `code` is a large binary buffer. See [`crate::error_fmt`].
    fn context_lines(&self) -> Vec<String> {
        crate::error_fmt::context_lines(self.code, self.loc)
    }

    /// Write the location as `line L, column C (offset N)`, or just `offset N`
    /// when the line is too far in to resolve.
    ///
    /// `column` is in display widths of the rendered line (it matches the
    /// `^--- here` pointer); `offset` is the absolute index into the input
    /// (bytes for a byte cursor, elements for any other cursor).
    fn write_location(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.readable_position() {
            Some(pos) => write!(
                f,
                "line {}, column {} (offset {})",
                pos.line, pos.column, self.loc
            ),
            None => write!(f, "offset {}", self.loc),
        }
    }

    /// Write the blank separator line and the context snippet
    fn write_context(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        for line in self.context_lines() {
            writeln!(f, "{}", line)?;
        }
        Ok(())
    }
}

/// What a leaf parser expected and what it found instead
///
/// Holds plain data so constructing it never allocates; the message is only
/// built when the error is displayed.
#[derive(Debug, Clone)]
pub enum Expected {
    /// A specific byte
    Byte { expected: u8, found: u8 },
    /// A byte in an inclusive range
    ByteInRange { start: u8, end: u8, found: u8 },
    /// A specific character
    Char { expected: char, found: char },
    /// A character of a string being matched; `found` is `None` at end of input
    CharInString {
        expected: char,
        found: Option<char>,
        string: Cow<'static, str>,
    },
}

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Printable ASCII as-is, everything else escaped (`\n`, `\x80`, ...)
        let show = std::ascii::escape_default;
        match self {
            Expected::Byte { expected, found } => write!(
                f,
                "expected byte 0x{:02X} ('{}'), found 0x{:02X} ('{}')",
                expected,
                show(*expected),
                found,
                show(*found)
            ),
            Expected::ByteInRange { start, end, found } => write!(
                f,
                "expected byte in range 0x{:02X}-0x{:02X} ('{}'-'{}'), found 0x{:02X} ('{}')",
                start,
                end,
                show(*start),
                show(*end),
                found,
                show(*found)
            ),
            Expected::Char { expected, found } => {
                write!(f, "expected {:?}, found {:?}", expected, found)
            }
            Expected::CharInString {
                expected,
                found: Some(found),
                string,
            } => write!(
                f,
                "expected {:?}, found {:?} while matching {:?}",
                expected, found, string
            ),
            Expected::CharInString {
                expected,
                found: None,
                string,
            } => write!(
                f,
                "expected {:?}, but reached end of input while matching {:?}",
                expected, string
            ),
        }
    }
}

#[derive(Debug)]
pub enum ParsicombError<'code, T: Atomic = u8> {
    UnexpectedEndOfFile(CodeLoc<'code, T>),
    AlreadyAtEndOfFile(CodeLoc<'code, T>),
    CannotReadValueAtEof(CodeLoc<'code, T>),
    SyntaxError {
        message: Cow<'static, str>,
        loc: CodeLoc<'code, T>,
    },
    /// A leaf parser found something other than what it expected
    Expected {
        expected: Expected,
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
            ParsicombError::Expected { expected, .. } => write!(f, ": {}", expected)?,
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
    fn test_codeloc_readable_position_eos() {
        let data = b"line1\nline2";
        let loc = CodeLoc::new(data, 11); // Position 11 = past end
        let pos = loc.readable_position().unwrap();

        // Should be on line 2, at column 5 (past "line2")
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 5);
    }

    #[test]
    fn test_codeloc_context_lines_eos() {
        let data = b"line1\nline2";
        let loc = CodeLoc::new(data, 11); // Position 11 = past end

        // Should not panic when generating context lines
        let context = loc.context_lines();
        assert!(!context.is_empty());

        // Should contain the line content
        let context_str = context.join("\n");
        assert!(context_str.contains("line2"));
    }

    #[test]
    fn test_codeloc_context_lines_empty_eos() {
        let data = b"";
        let loc = CodeLoc::new(data, 0);

        // Should not panic even with empty data
        let _context = loc.context_lines();
        // May be empty or contain minimal context, but shouldn't panic
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
        let code = vec![b'x'; crate::error_fmt::MAX_POSITION_SCAN + 8];
        let error: ParsicombError =
            ParsicombError::UnexpectedEndOfFile(CodeLoc::new(&code, code.len()));
        let first = error.to_string().lines().next().unwrap().to_string();
        assert_eq!(
            first,
            format!("Unexpected end of file at offset {}", code.len())
        );
    }
}
