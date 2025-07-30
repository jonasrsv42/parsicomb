use crate::atomic::Atomic;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;

/// Trait for errors that can report their byte position in the input
/// This enables selecting the error that progressed furthest when multiple parsers fail
pub trait ErrorLeaf: Error {
    /// Returns the byte position where this error occurred
    fn byte_position(&self) -> usize;
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
/// use parsicomb::error::{ErrorLeaf, ErrorNode};
/// use std::error::Error;
/// use std::fmt;
///
/// // Your custom error type
/// #[derive(Debug)]
/// struct MyError {
///     position: usize,
///     message: String,
/// }
///
/// impl fmt::Display for MyError {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", self.message)
///     }
/// }
///
/// impl Error for MyError {}
///
/// // Implement ErrorLeaf
/// impl ErrorLeaf for MyError {
///     fn byte_position(&self) -> usize {
///         self.position
///     }
/// }
///
/// // Implement ErrorNode (converts to itself since it's already a terminal type)
/// impl<'a> ErrorNode<'a> for MyError {
///     fn likely_error(&self) -> &dyn ErrorLeaf {
///         self
///     }
/// }
/// ```
pub trait ErrorNode<'code>: std::fmt::Display + std::fmt::Debug {
    /// Flatten nested error structures and return the likely error that made it furthest
    fn likely_error(&self) -> &dyn ErrorLeaf;
}

#[derive(Debug)]
pub struct ReadablePosition {
    pub line: usize,
    pub byte_offset: usize,
}

#[derive(Debug)]
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
            if element == T::NEWLINE {
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
    AlreadyAtEndOfFile,
    CannotReadValueAtEof,
    SyntaxError {
        message: Cow<'static, str>,
        loc: CodeLoc<'code, T>,
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
            ParsicombError::AlreadyAtEndOfFile => {
                write!(f, "Already at end of file")
            }
            ParsicombError::CannotReadValueAtEof => {
                write!(f, "Cannot read value at EOF")
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
        }
    }
}

impl<'code, T: Atomic> Error for ParsicombError<'code, T> {}

impl<'code, T: Atomic> ParsicombError<'code, T> {
    /// Returns the position where this error occurred
    pub fn position(&self) -> usize {
        match self {
            ParsicombError::UnexpectedEndOfFile(code_loc) => code_loc.position(),
            ParsicombError::AlreadyAtEndOfFile => 0, // Assume EOF is at end
            ParsicombError::CannotReadValueAtEof => 0, // Assume EOF is at end
            ParsicombError::SyntaxError { loc, .. } => loc.position(),
        }
    }
}

impl<'code, T: Atomic> ErrorLeaf for ParsicombError<'code, T> {
    fn byte_position(&self) -> usize {
        self.position()
    }
}

// ParsicombError implements ErrorBranch (converts to itself since it's a terminal type)
impl<'code, T: Atomic> ErrorNode<'code> for ParsicombError<'code, T>
where
    T: 'code,
{
    fn likely_error(&self) -> &dyn ErrorLeaf {
        self // Already the base type
    }
}
