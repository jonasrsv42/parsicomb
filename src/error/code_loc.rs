use super::render::ReadablePosition;
use crate::atomic::Atomic;
use std::fmt;

/// A position in the input, together with the input it points into
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
    /// Delegates to [`super::render`]; see there for details.
    fn readable_position(&self) -> Option<ReadablePosition> {
        super::render::readable_position(self.code, self.loc)
    }

    /// Get lines of context around the error position.
    ///
    /// Returns up to 2 lines before and after the error line. Only a bounded
    /// window around the error is materialized, so this stays cheap even when
    /// `code` is a large binary buffer. See [`super::render`].
    fn context_lines(&self) -> Vec<String> {
        super::render::context_lines(self.code, self.loc)
    }

    /// Write the location as `line L, column C (offset N)`, or just `offset N`
    /// when the line is too far in to resolve.
    ///
    /// `column` is in display widths of the rendered line (it matches the
    /// `^--- here` pointer); `offset` is the absolute index into the input
    /// (bytes for a byte cursor, elements for any other cursor).
    pub(super) fn write_location(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    pub(super) fn write_context(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        for line in self.context_lines() {
            writeln!(f, "{}", line)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
