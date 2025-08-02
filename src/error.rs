use crate::atomic::Atomic;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;

/// Trait for errors that can report their location in the input
/// This enables selecting the error that progressed furthest when multiple parsers fail
pub trait ErrorLeaf<'code>: Error {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// Returns the location where this error occurred
    fn loc(&self) -> CodeLoc<'code, Self::Element>;
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
/// // Your custom error type
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
/// // Implement ErrorLeaf
/// impl<'code> ErrorLeaf<'code> for MyError<'code> {
///     type Element = u8;
///     
///     fn loc(&self) -> CodeLoc<'code, Self::Element> {
///         CodeLoc::new(self.code, self.position)
///     }
/// }
///
/// // Implement ErrorNode (converts to itself since it's already a terminal type)
/// impl<'code> ErrorNode<'code> for MyError<'code> {
///     type Element = u8;
///     
///     fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = Self::Element> {
///         self
///     }
/// }
/// ```
pub trait ErrorNode<'code>: std::fmt::Display + std::fmt::Debug {
    /// The element type used in the source code (e.g., u8 for bytes)
    type Element: Atomic;

    /// Flatten nested error structures and return the likely error that made it furthest
    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = Self::Element>;
}

#[derive(Debug)]
pub struct ReadablePosition {
    pub line: usize,
    pub byte_offset: usize,
}

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
}

impl<'code, T: Atomic> CodeLoc<'code, T> {
    /// Calculate line number and element offset within that line
    ///
    /// Note: We return element offset instead of column number because column
    /// calculation is complex - it depends on:
    /// - Text encoding (UTF-8 can have multi-byte characters)
    /// - Rendering context (tabs can be 2, 4, 8 spaces)
    /// - Terminal width and line wrapping
    /// - Zero-width characters, combining characters, etc.
    ///
    /// Element offset within the line is unambiguous and useful for debugging.
    fn readable_position(&self) -> ReadablePosition {
        let mut line = 1;
        let mut line_start = 0;

        for (i, &element) in self.code.iter().enumerate() {
            if i >= self.loc {
                break;
            }
            if element.is_newline() {
                line += 1;
                line_start = i + 1;
            }
        }

        let byte_offset = self.loc - line_start;
        ReadablePosition { line, byte_offset }
    }

    /// Get lines of context around the error position
    /// Returns up to 2 lines before and after the error line
    fn context_lines(&self) -> Vec<String> {
        let pos = self.readable_position();
        let mut lines = Vec::new();
        let mut current_line = 1;
        let mut line_start = 0;

        // Convert to string for easier line handling
        let text = T::format_slice(&self.code);

        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                // Check if this line is within our context window
                if current_line >= pos.line.saturating_sub(2) && current_line <= pos.line + 2 {
                    let line_content = &text[line_start..i];
                    let prefix = if current_line == pos.line {
                        format!("  > {} | ", current_line)
                    } else {
                        format!("    {} | ", current_line)
                    };
                    lines.push(format!("{}{}", prefix, line_content));

                    // Add error pointer for the error line
                    if current_line == pos.line {
                        let pointer_offset = prefix.len() + pos.byte_offset;
                        let pointer = format!("{}^--- here", " ".repeat(pointer_offset));
                        lines.push(pointer);
                    }
                }

                current_line += 1;
                line_start = i + 1;
            }
        }

        // Handle last line if no trailing newline
        if line_start < text.len()
            && current_line >= pos.line.saturating_sub(2)
            && current_line <= pos.line + 2
        {
            let line_content = &text[line_start..];
            let prefix = if current_line == pos.line {
                format!("  > {} | ", current_line)
            } else {
                format!("    {} | ", current_line)
            };
            lines.push(format!("{}{}", prefix, line_content));

            if current_line == pos.line {
                let pointer_offset = prefix.len() + pos.byte_offset;
                let pointer = format!("{}^--- here", " ".repeat(pointer_offset));
                lines.push(pointer);
            }
        }

        lines
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
    /// Wrapped error from another parser combinator
    WrappedError {
        inner: Box<dyn ErrorNode<'code, Element = T> + 'code>,
    },
}

impl<'code, T: Atomic> fmt::Display for ParsicombError<'code, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParsicombError::UnexpectedEndOfFile(code_loc) => {
                let pos = code_loc.readable_position();
                writeln!(
                    f,
                    "Unexpected end of file at line {}, byte offset {} (absolute position: {})",
                    pos.line, pos.byte_offset, code_loc.loc
                )?;
                writeln!(f)?;
                for line in code_loc.context_lines() {
                    writeln!(f, "{}", line)?;
                }
                Ok(())
            }
            ParsicombError::AlreadyAtEndOfFile(code_loc) => {
                let pos = code_loc.readable_position();
                writeln!(
                    f,
                    "Already at end of file at line {}, byte offset {} (absolute position: {})",
                    pos.line, pos.byte_offset, code_loc.loc
                )?;
                writeln!(f)?;
                for line in code_loc.context_lines() {
                    writeln!(f, "{}", line)?;
                }
                Ok(())
            }
            ParsicombError::CannotReadValueAtEof(code_loc) => {
                let pos = code_loc.readable_position();
                writeln!(
                    f,
                    "Cannot read value at EOF at line {}, byte offset {} (absolute position: {})",
                    pos.line, pos.byte_offset, code_loc.loc
                )?;
                writeln!(f)?;
                for line in code_loc.context_lines() {
                    writeln!(f, "{}", line)?;
                }
                Ok(())
            }
            ParsicombError::SyntaxError { message, loc } => {
                let pos = loc.readable_position();
                writeln!(
                    f,
                    "Syntax error at line {}, byte offset {}: {}",
                    pos.line, pos.byte_offset, message
                )?;
                writeln!(f)?;
                for line in loc.context_lines() {
                    writeln!(f, "{}", line)?;
                }
                Ok(())
            }
            ParsicombError::WrappedError { inner } => {
                // Delegate to the inner error's likely_error for display
                let likely = inner.likely_error();
                write!(f, "{}", likely)
            }
        }
    }
}

impl<'code, T: Atomic> Error for ParsicombError<'code, T> {}

impl<'code, T: Atomic> ParsicombError<'code, T> {
    /// Wrap an ErrorNode in a ParsicombError
    pub fn wrap(error: impl ErrorNode<'code, Element = T> + 'code) -> Self {
        ParsicombError::WrappedError {
            inner: Box::new(error),
        }
    }

    /// Returns the position where this error occurred
    pub fn position(&self) -> usize {
        match self {
            ParsicombError::UnexpectedEndOfFile(code_loc) => code_loc.position(),
            ParsicombError::AlreadyAtEndOfFile(code_loc) => code_loc.position(),
            ParsicombError::CannotReadValueAtEof(code_loc) => code_loc.position(),
            ParsicombError::SyntaxError { loc, .. } => loc.position(),
            ParsicombError::WrappedError { inner } => {
                // Delegate to the wrapped error's likely_error
                inner.likely_error().loc().position()
            }
        }
    }
}

impl<'code, T: Atomic> ErrorLeaf<'code> for ParsicombError<'code, T> {
    type Element = T;

    fn loc(&self) -> CodeLoc<'code, Self::Element> {
        match self {
            ParsicombError::UnexpectedEndOfFile(code_loc) => *code_loc,
            ParsicombError::AlreadyAtEndOfFile(code_loc) => *code_loc,
            ParsicombError::CannotReadValueAtEof(code_loc) => *code_loc,
            ParsicombError::SyntaxError { loc, .. } => *loc,
            ParsicombError::WrappedError { inner } => {
                // Get the likely error and call loc on it
                inner.likely_error().loc()
            }
        }
    }
}

// ParsicombError implements ErrorNode (converts to itself since it's a terminal type)
impl<'code, T: Atomic> ErrorNode<'code> for ParsicombError<'code, T>
where
    T: 'code,
{
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<'code, Element = Self::Element> {
        self // Already the base type
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
        let pos = loc.readable_position();

        // Should be on line 2, with byte offset 5 (past "line2")
        assert_eq!(pos.line, 2);
        assert_eq!(pos.byte_offset, 5);
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
        assert!(display_str.contains("byte offset 0"));
    }
}
