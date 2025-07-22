use std::borrow::Cow;
use std::error::Error;
use std::fmt;

/// Trait for errors that can report their byte position in the input
/// This enables selecting the error that progressed furthest when multiple parsers fail
pub trait ErrorPosition {
    /// Returns the byte position where this error occurred
    fn byte_position(&self) -> usize;
}

#[derive(Debug)]
pub struct ReadablePosition {
    pub line: usize,
    pub byte_offset: usize,
}

#[derive(Debug)]
pub struct CodeLoc<'code> {
    code: &'code [u8],
    /// The byte position in `code` where the cursor encountered an error
    loc: usize,
}

impl<'code> CodeLoc<'code> {
    pub fn new(code: &'code [u8], loc: usize) -> Self {
        Self { code, loc }
    }

    /// Calculate line number and byte offset within that line
    ///
    /// Note: We return byte offset instead of column number because column
    /// calculation is complex - it depends on:
    /// - Text encoding (UTF-8 can have multi-byte characters)
    /// - Rendering context (tabs can be 2, 4, 8 spaces)
    /// - Terminal width and line wrapping
    /// - Zero-width characters, combining characters, etc.
    ///
    /// Byte offset within the line is unambiguous and useful for debugging.
    fn readable_position(&self) -> ReadablePosition {
        let mut line = 1;
        let mut line_start = 0;

        for (i, &byte) in self.code.iter().enumerate() {
            if i >= self.loc {
                break;
            }
            if byte == b'\n' {
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
        let text = String::from_utf8_lossy(&self.code);

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
pub enum ParsicombError<'code> {
    UnexpectedEndOfFile(CodeLoc<'code>),
    AlreadyAtEndOfFile,
    CannotReadValueAtEof,
    SyntaxError {
        message: Cow<'static, str>,
        loc: CodeLoc<'code>,
    },
}

impl<'code> fmt::Display for ParsicombError<'code> {
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

impl<'code> Error for ParsicombError<'code> {}

impl<'code> ParsicombError<'code> {
    /// Returns the byte offset where this error occurred
    pub fn byte_offset(&self) -> usize {
        match self {
            ParsicombError::UnexpectedEndOfFile(code_loc) => code_loc.loc,
            ParsicombError::AlreadyAtEndOfFile => 0, // Assume EOF is at end
            ParsicombError::CannotReadValueAtEof => 0, // Assume EOF is at end
            ParsicombError::SyntaxError { loc, .. } => loc.loc,
        }
    }
}

impl<'code> ErrorPosition for ParsicombError<'code> {
    fn byte_position(&self) -> usize {
        self.byte_offset()
    }
}
