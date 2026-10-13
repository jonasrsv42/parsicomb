use super::{CodeLoc, ErrorLeaf, ErrorNode, Expected, Progress};
use crate::atomic::Atomic;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;

/// The crate's concrete error type: leaf parsers and cursors return it, and
/// every combinator's error converts into it with `ParsicombError::from`
#[derive(Debug)]
#[non_exhaustive]
pub enum ParsicombError<'code, T: Atomic = u8> {
    /// Tried to read an element at the end of input
    EndOfInput(CodeLoc<'code, T>),
    /// A real problem in the input, described by a free-form message
    Invalid {
        /// What went wrong
        message: Cow<'static, str>,
        /// Where it went wrong
        loc: CodeLoc<'code, T>,
    },
    /// A leaf parser found something other than what it expected
    Expected {
        /// What the parser expected
        expected: Expected<T>,
        /// Where it looked
        loc: CodeLoc<'code, T>,
    },
    /// Wrapped error from another parser combinator; built by
    /// [`ParsicombError::wrap`], which keeps `progress` in sync with `inner`
    #[non_exhaustive]
    WrappedError {
        /// The wrapped error
        inner: Box<dyn ErrorNode<Element = T> + Send + Sync + 'code>,
        /// `inner`'s progress, computed once when wrapping
        progress: Progress,
    },
}

impl<'code, T: Atomic> fmt::Display for ParsicombError<'code, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (headline, loc) = match self {
            ParsicombError::EndOfInput(loc) => ("Unexpected end of input", loc),
            ParsicombError::Invalid { loc, .. } => ("Syntax error", loc),
            ParsicombError::Expected { loc, .. } => ("Syntax error", loc),
            ParsicombError::WrappedError { inner, .. } => {
                return write!(f, "{}", inner.likely_error());
            }
        };

        write!(f, "{} at ", headline)?;
        let position = loc.write_location(f)?;
        match self {
            ParsicombError::Invalid { message, .. } => write!(f, ": {}", message)?,
            ParsicombError::Expected { expected, loc } => {
                write!(f, ": ")?;
                expected.write_message(loc, f)?;
            }
            _ => {}
        }
        writeln!(f)?;
        loc.write_context(f, position)
    }
}

impl<'code, T: Atomic> Error for ParsicombError<'code, T> {}

impl<'code, T: Atomic> ParsicombError<'code, T> {
    /// Wrap any ErrorNode in a ParsicombError, allocating a box
    ///
    /// Errors of this crate's combinators convert through [`From`] (e.g.
    /// `.map_err(ParsicombError::from)` or `?`), allocating only for failures
    /// under `commit`, `attempt`, `max_depth` or `skip_then`; `wrap` is for
    /// other error types.
    pub fn wrap(error: impl ErrorNode<Element = T> + Send + Sync + 'code) -> Self {
        ParsicombError::WrappedError {
            progress: error.progress(),
            inner: Box::new(error),
        }
    }

    /// Returns the position where this error occurred
    pub fn position(&self) -> usize {
        self.progress().position()
    }
}

impl<'code, T: Atomic> ErrorLeaf for ParsicombError<'code, T> {
    type Element = T;

    fn loc(&self) -> CodeLoc<'_, Self::Element> {
        match self {
            ParsicombError::EndOfInput(loc)
            | ParsicombError::Invalid { loc, .. }
            | ParsicombError::Expected { loc, .. } => *loc,
            ParsicombError::WrappedError { inner, .. } => inner.likely_error().loc(),
        }
    }
}

impl<'code, T: Atomic> ErrorNode for ParsicombError<'code, T> {
    type Element = T;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        match self {
            ParsicombError::WrappedError { inner, .. } => inner.likely_error(),
            _ => self,
        }
    }

    fn progress(&self) -> Progress {
        match self {
            ParsicombError::WrappedError { progress, .. } => *progress,
            ParsicombError::Expected { expected, loc } => expected.progress(loc.position()),
            ParsicombError::EndOfInput(loc) | ParsicombError::Invalid { loc, .. } => {
                Progress::at(loc.position())
            }
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
        let error = ParsicombError::EndOfInput(loc);

        // Should not panic when displaying
        let display_str = format!("{}", error);
        assert!(display_str.contains("Unexpected end of input"));

        // Should handle position calculation correctly
        assert_eq!(loc.position(), 0);
    }
    #[test]
    fn test_codeloc_eos_single_byte() {
        let data = b"a";
        let loc = CodeLoc::new(data, 1); // Position 1 = past end
        let error = ParsicombError::EndOfInput(loc);

        // Should not panic when displaying
        let display_str = format!("{}", error);
        assert!(display_str.contains("Unexpected end of input"));

        // Should handle position calculation correctly
        assert_eq!(loc.position(), 1);
    }
    #[test]
    fn test_codeloc_eos_multiline() {
        let data = b"hello\nworld";
        let loc = CodeLoc::new(data, 11); // Position 11 = past end
        let error = ParsicombError::EndOfInput(loc);

        // Should not panic when displaying
        let display_str = format!("{}", error);
        assert!(display_str.contains("Unexpected end of input"));

        // Should handle position calculation correctly
        assert_eq!(loc.position(), 11);
    }
    #[test]
    fn test_codeloc_eos_line_ending() {
        let data = b"hello\n";
        let loc = CodeLoc::new(data, 6); // Position 6 = past end (after newline)
        let error = ParsicombError::Invalid {
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
        let error = ParsicombError::EndOfInput(loc);

        let display_str = format!("{}", error);
        println!("EOS Error Display:\n{}", display_str);

        // Verify it contains expected parts
        assert!(display_str.contains("Unexpected end of input"));
        assert!(display_str.contains("line 2"));
        assert!(display_str.contains("world"));
    }
    #[test]
    fn test_eos_after_newline_display() {
        // Test EOS position right after a newline
        let data = b"hello\n";
        let loc = CodeLoc::new(data, 6); // Position 6 = past end (after newline)
        let error = ParsicombError::EndOfInput(loc);

        let display_str = format!("{}", error);
        println!("EOS After Newline:\n{}", display_str);

        // Should be on line 2, column 1
        assert!(display_str.contains("line 2"));
        assert!(display_str.contains("column 1"));
    }
    #[test]
    fn test_location_reports_line_column_and_offset() {
        let code = "ab\ncé?".as_bytes();
        let error: ParsicombError = ParsicombError::Invalid {
            message: "boom".into(),
            loc: CodeLoc::new(code, code.len() - 1),
        };
        // column counts 'é' once; offset counts its two bytes
        let first = error.to_string().lines().next().unwrap().to_string();
        assert_eq!(first, "Syntax error at line 2, column 3 (offset 6): boom");
    }
    #[test]
    fn test_location_far_offset_falls_back_to_offset_only() {
        let code = vec![b'x'; crate::error::render::MAX_POSITION_SCAN + 8];
        let error: ParsicombError = ParsicombError::EndOfInput(CodeLoc::new(&code, code.len()));
        let first = error.to_string().lines().next().unwrap().to_string();
        assert_eq!(
            first,
            format!("Unexpected end of input at offset {}", code.len())
        );
    }

    #[test]
    fn test_error_is_send_and_sync() {
        fn assert_send_sync<E: Send + Sync>() {}
        assert_send_sync::<ParsicombError<'static>>();
        assert_send_sync::<ParsicombError<'static, u32>>();
    }

    #[test]
    fn test_invalid_bytes_are_described_one_at_a_time() {
        use crate::Parser;
        use crate::byte::is_byte;
        use crate::cursors::ByteCursor;

        for input in [&b"\xF0\x9F A"[..], b"\xE2\x82"] {
            let message = is_byte(b'x')
                .parse(ByteCursor::new(input))
                .unwrap_err()
                .to_string();
            assert!(
                message.contains(&format!("found byte 0x{:02X}", input[0])),
                "got {message}"
            );
        }
        let message = is_byte(b'x')
            .parse(ByteCursor::new("€".as_bytes()))
            .unwrap_err()
            .to_string();
        assert!(message.contains(r#"found "€""#), "got {message}");
    }

    #[test]
    fn test_wrapped_position_comes_from_the_inner_error() {
        let inner = ParsicombError::Invalid {
            message: "boom".into(),
            loc: CodeLoc::new(b"abcdef", 5),
        };
        let wrapped = ParsicombError::wrap(ParsicombError::wrap(inner));
        assert_eq!(wrapped.position(), 5);
        assert_eq!(wrapped.likely_error().loc().position(), 5);
        assert_eq!(wrapped.progress(), Progress::at(5));
    }

    #[test]
    fn test_debug_does_not_print_the_input() {
        use crate::Parser;
        use crate::byte::is_byte;
        use crate::combinator::PositionExt;
        use crate::cursors::ByteCursor;

        let input = vec![b'z'; 1 << 20];
        let error = is_byte(b'a').parse(ByteCursor::new(&input)).unwrap_err();
        let debug = format!("{error:?}");
        assert!(debug.len() < 200, "got {} bytes", debug.len());
        assert!(debug.contains("input_len: 1048576"), "got {debug}");

        let cursor = ByteCursor::new(&input);
        assert!(format!("{cursor:?}").len() < 100);
        let ((_, span), _) = crate::byte::byte().with_position().parse(cursor).unwrap();
        assert_eq!(
            format!("{span:?}"),
            "Span { start: 0, end: 1, input_len: 1048576 }"
        );
    }
}
