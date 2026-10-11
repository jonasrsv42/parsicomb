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
    /// A location at index `loc` of `code`
    pub fn new(code: &'code [T], loc: usize) -> Self {
        Self { code, loc }
    }

    /// Index into the input
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

    /// Write the location as `line L, column C (offset N)`, or just `offset N`
    /// when the line is too far in to resolve, and return the resolved
    /// position for [`write_context`](Self::write_context).
    ///
    /// `column` counts characters of the rendered line from its start (it
    /// matches the `^--- here` pointer unless the snippet had to cut a very
    /// long line); `offset` is the absolute index into the input (bytes for a
    /// byte cursor, elements for any other cursor).
    pub(super) fn write_location(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> Result<Option<ReadablePosition>, fmt::Error> {
        let position = self.readable_position();
        match position {
            Some(pos) => write!(
                f,
                "line {}, column {} (offset {})",
                pos.line, pos.column, self.loc
            )?,
            None => write!(f, "offset {}", self.loc)?,
        }
        Ok(position)
    }

    /// Write the blank separator line and the context snippet around the
    /// position [`write_location`](Self::write_location) resolved
    pub(super) fn write_context(
        &self,
        f: &mut fmt::Formatter<'_>,
        position: Option<ReadablePosition>,
    ) -> fmt::Result {
        writeln!(f)?;
        if let Some(pos) = position {
            for line in super::render::context_lines_at(self.code, self.loc, pos) {
                writeln!(f, "{}", line)?;
            }
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
        let context = super::super::render::context_lines(loc.code(), loc.position());
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
        let _context = super::super::render::context_lines(loc.code(), loc.position());
        // May be empty or contain minimal context, but shouldn't panic
    }
}
