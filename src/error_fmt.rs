//! Bounded formatting helpers for parser error locations.
//!
//! These are the pure, position-only routines behind `CodeLoc`'s `Display`
//! output: computing a human-readable line/column and a small context snippet
//! around an error position.
//!
//! The important property here is that **rendering is bounded** — it scans and
//! materializes at most [`CONTEXT_RADIUS`] elements on each side of the error
//! position, never the whole input. A parse failure on a multi-megabyte binary
//! blob therefore does not stringify the entire buffer (which previously made
//! formatting a one-line error O(input size) with a full-buffer allocation).
//!
//! For normal source-sized inputs the window comfortably covers the full ±2
//! lines of context, so the rendered output is identical to the unwindowed
//! version. The unit tests below pin that against a brute-force reference.

use crate::atomic::Atomic;

/// Maximum number of input elements scanned and rendered on each side of the
/// error position when building the context snippet.
pub(crate) const CONTEXT_RADIUS: usize = 512;

/// A human-readable position: 1-based line and a column expressed in display
/// widths (see [`Atomic::display_width`]) from the start of that line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadablePosition {
    pub line: usize,
    /// Character offset within the line using display widths.
    pub byte_offset: usize,
}

/// Calculate the line number and display-width column of `loc` within `code`.
///
/// This counts newlines from the start of the input, so it is O(loc) — but it
/// is allocation-free (plain element iteration) and only runs when an error is
/// actually rendered.
pub(crate) fn readable_position<T: Atomic>(code: &[T], loc: usize) -> ReadablePosition {
    let loc = loc.min(code.len());
    let mut line = 1;
    let mut line_start_element = 0;

    for (i, element) in code.iter().enumerate() {
        if i >= loc {
            break;
        }
        if element.is_newline() {
            line += 1;
            line_start_element = i + 1;
        }
    }

    let char_offset = code[line_start_element..loc]
        .iter()
        .map(|element| element.display_width())
        .sum::<usize>();

    ReadablePosition {
        line,
        byte_offset: char_offset,
    }
}

/// Compute the byte window of `code` to materialize for the context snippet,
/// bounded to [`CONTEXT_RADIUS`] elements on each side of the error.
///
/// Returns `(window_start, window_end, first_line)`, where `first_line` is the
/// line number of `code[window_start]`. All scanning here is bounded by the
/// radius, so it never walks the whole input.
fn context_window<T: Atomic>(code: &[T], loc: usize, err_line: usize) -> (usize, usize, usize) {
    let len = code.len();
    let loc = loc.min(len);

    // Walk backward to the start of the line two lines above the error
    // (the 3rd newline boundary back), capped at CONTEXT_RADIUS elements. In
    // binary input with no newlines this simply stops at the radius cap.
    let back_cap = loc.saturating_sub(CONTEXT_RADIUS);
    let mut nl_back = [0usize; 3];
    let mut nb = 0;
    let mut i = loc;
    while i > back_cap && nb < 3 {
        if code[i - 1].is_newline() {
            nl_back[nb] = i - 1;
            nb += 1;
        }
        i -= 1;
    }
    let win_start = if nb == 3 { nl_back[2] + 1 } else { i };
    // Newlines remaining strictly inside [win_start, loc) tell us how many lines
    // below `first_line` the error line sits.
    let inside = (0..nb).filter(|&k| nl_back[k] >= win_start).count();
    let first_line = err_line - inside;

    // Walk forward to the end of the line two lines below the error
    // (the 3rd newline boundary forward), capped at CONTEXT_RADIUS elements.
    let fwd_cap = (loc + CONTEXT_RADIUS).min(len);
    let mut nf = 0;
    let mut j = loc;
    while j < fwd_cap && nf < 3 {
        if code[j].is_newline() {
            nf += 1;
        }
        j += 1;
    }

    (win_start, j, first_line)
}

/// Get lines of context around the error position. Returns up to 2 lines before
/// and after the error line, each prefixed with its line number, plus a
/// `^--- here` pointer under the error line.
///
/// Only the bounded window from [`context_window`] is materialized; the rest of
/// the input is never stringified.
pub(crate) fn context_lines<T: Atomic>(code: &[T], loc: usize) -> Vec<String> {
    let len = code.len();
    let loc = loc.min(len);
    let pos = readable_position(code, loc);
    let (win_start, win_end, first_line) = context_window(code, loc, pos.line);

    // Materialize only the bounded window, not the whole input.
    let text = T::format_slice(&code[win_start..win_end]);

    let mut lines = Vec::new();
    let mut current_line = first_line;
    let mut line_start = 0; // byte offset within `text`

    let emit =
        |current_line: usize, line_start: usize, line_end: usize, lines: &mut Vec<String>| {
            // Same ±2 line context window as before (saturating on the low end).
            if current_line + 2 < pos.line || current_line > pos.line + 2 {
                return;
            }
            let is_err = current_line == pos.line;
            let prefix = if is_err {
                format!("  > {} | ", current_line)
            } else {
                format!("    {} | ", current_line)
            };
            lines.push(format!("{}{}", prefix, &text[line_start..line_end]));

            if is_err {
                // Column of the error within the (windowed) line, in display
                // widths. Bounded by the window, so no O(input) work and no
                // pathological `" ".repeat(huge_offset)` allocation.
                let line_abs_start = win_start + line_start;
                let col: usize = code[line_abs_start..loc]
                    .iter()
                    .map(|e| e.display_width())
                    .sum();
                let pointer_offset = prefix.len() + col;
                lines.push(format!("{}^--- here", " ".repeat(pointer_offset)));
            }
        };

    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            emit(current_line, line_start, i, &mut lines);
            current_line += 1;
            line_start = i + 1;
        }
    }

    // Handle the last line if there is no trailing newline (this is also the
    // binary / no-newline case, where the whole window is one "line").
    if line_start < text.len() {
        emit(current_line, line_start, text.len(), &mut lines);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force reference: the original unwindowed algorithm. Used to prove
    /// the windowed implementation produces identical output whenever the whole
    /// input fits inside the context window (i.e. normal source-sized inputs).
    fn context_lines_reference(code: &[u8], loc: usize) -> Vec<String> {
        let pos = readable_position(code, loc);
        let mut lines = Vec::new();
        let mut current_line = 1;
        let mut line_start = 0;
        let text = String::from_utf8_lossy(code).to_string();

        let emit = |current_line: usize,
                    line_start: usize,
                    line_end: usize,
                    text: &str,
                    lines: &mut Vec<String>| {
            if current_line >= pos.line.saturating_sub(2) && current_line <= pos.line + 2 {
                let prefix = if current_line == pos.line {
                    format!("  > {} | ", current_line)
                } else {
                    format!("    {} | ", current_line)
                };
                lines.push(format!("{}{}", prefix, &text[line_start..line_end]));
                if current_line == pos.line {
                    let pointer_offset = prefix.len() + pos.byte_offset;
                    lines.push(format!("{}^--- here", " ".repeat(pointer_offset)));
                }
            }
        };

        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                emit(current_line, line_start, i, &text, &mut lines);
                current_line += 1;
                line_start = i + 1;
            }
        }
        if line_start < text.len() {
            emit(current_line, line_start, text.len(), &text, &mut lines);
        }
        lines
    }

    #[test]
    fn readable_position_basic() {
        let code = b"abc\ndef\nghi";
        // position of 'e' (index 5) is line 2, column 1
        let p = readable_position(code, 5);
        assert_eq!(p.line, 2);
        assert_eq!(p.byte_offset, 1);
    }

    #[test]
    fn readable_position_start() {
        let p = readable_position(b"hello", 0);
        assert_eq!(p.line, 1);
        assert_eq!(p.byte_offset, 0);
    }

    #[test]
    fn matches_reference_single_line() {
        let code = b"let x = ;";
        for loc in 0..=code.len() {
            assert_eq!(
                context_lines(code, loc),
                context_lines_reference(code, loc),
                "mismatch at loc {loc}"
            );
        }
    }

    #[test]
    fn matches_reference_multi_line() {
        let code = b"line one\nline two\nline three\nline four\nline five\nline six";
        for loc in 0..=code.len() {
            assert_eq!(
                context_lines(code, loc),
                context_lines_reference(code, loc),
                "mismatch at loc {loc}"
            );
        }
    }

    #[test]
    fn matches_reference_trailing_newline() {
        let code = b"a\nb\nc\n";
        for loc in 0..=code.len() {
            assert_eq!(
                context_lines(code, loc),
                context_lines_reference(code, loc),
                "mismatch at loc {loc}"
            );
        }
    }

    #[test]
    fn matches_reference_blank_lines() {
        let code = b"a\n\n\nb\n\nc";
        for loc in 0..=code.len() {
            assert_eq!(
                context_lines(code, loc),
                context_lines_reference(code, loc),
                "mismatch at loc {loc}"
            );
        }
    }

    #[test]
    fn first_line_number_is_correct() {
        // Error on line 4 of 6; context should start at line 2 ("    2 | ...").
        let code = b"l1\nl2\nl3\nl4\nl5\nl6";
        let loc = 9; // somewhere in "l4"
        assert_eq!(readable_position(code, loc).line, 4);
        let out = context_lines(code, loc);
        assert!(out[0].starts_with("    2 | "), "got {:?}", out[0]);
        // error line is line 4, rendered with the `>` marker
        assert!(out.iter().any(|l| l.starts_with("  > 4 | ")), "got {out:?}");
    }

    #[test]
    fn large_binary_is_bounded() {
        // 4 MiB, no newlines: the whole thing is "line 1". Rendering must only
        // touch a window around the error, so the output stays tiny.
        let code = vec![0xABu8; 4 << 20];
        let loc = code.len() - 1;
        let out = context_lines(&code, loc);

        let total: usize = out.iter().map(|l| l.len()).sum();
        // Bounded by ~2*RADIUS of content plus a pointer line, nowhere near 4 MiB.
        assert!(
            total <= 8 * CONTEXT_RADIUS,
            "context output not bounded: {total} bytes"
        );
        // The error line is line 1 and a pointer is emitted.
        assert!(out.iter().any(|l| l.starts_with("  > 1 | ")), "got {out:?}");
        assert!(out.iter().any(|l| l.contains("^--- here")), "got {out:?}");
    }

    #[test]
    fn long_single_line_is_truncated_not_dropped() {
        // A single line far longer than the radius: we still render a snippet
        // and a pointer, but bounded.
        let mut code = vec![b'x'; 3 * CONTEXT_RADIUS];
        let loc = code.len() / 2;
        code[loc] = b'?';
        let out = context_lines(&code, loc);
        let total: usize = out.iter().map(|l| l.len()).sum();
        assert!(total <= 6 * CONTEXT_RADIUS, "not bounded: {total}");
        assert!(out.iter().any(|l| l.contains("^--- here")));
    }

    #[test]
    fn loc_past_end_is_clamped() {
        let code = b"abc";
        // Should not panic even if loc is out of range.
        let _ = context_lines(code, 999);
        let p = readable_position(code, 999);
        assert_eq!(p.line, 1);
    }
}
