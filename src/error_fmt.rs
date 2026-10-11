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

/// Maximum distance (in elements, from the start of input) we will scan to
/// resolve a human-readable line/column.
///
/// Resolving line/column is inherently O(loc) — it must count newlines before
/// the error. Past this distance we don't try: an input this large is far
/// bigger than any normal source file (most likely a binary blob), line/column
/// would be meaningless, and the absolute byte offset is the useful location.
/// Error rendering is best-effort, so degrading to "byte offset only" here keeps
/// it O(1) without losing anything that matters.
///
/// Note this gates on the error *position*, not total input size: an error
/// early in a huge buffer is still cheap to resolve and gets full line/column.
pub(crate) const MAX_POSITION_SCAN: usize = 1 << 20; // 1 MiB

/// A human-readable position: 1-based line and a column expressed in display
/// widths (see [`Atomic::display_width`]) from the start of that line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadablePosition {
    pub line: usize,
    /// Column within the line (0-based), in display widths.
    pub column: usize,
}

/// Calculate the line number and display-width column of `loc` within `code`,
/// or `None` if `loc` is beyond [`MAX_POSITION_SCAN`] (too far in to scan).
///
/// When it does run it counts newlines from the start of the input (O(loc)), but
/// allocation-free and in a *single* pass: the column is accumulated in the same
/// loop that counts newlines (reset at each line break), rather than a second
/// scan over the error line. Only runs when an error is rendered.
pub(crate) fn readable_position<T: Atomic>(code: &[T], loc: usize) -> Option<ReadablePosition> {
    let loc = loc.min(code.len());
    if loc > MAX_POSITION_SCAN {
        return None;
    }

    let mut line = 1;
    let mut column = 0;

    for element in &code[..loc] {
        if element.is_newline() {
            line += 1;
            column = 0;
        } else {
            column += element.display_width();
        }
    }

    Some(ReadablePosition { line, column })
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
    let mut win_start = if nb == 3 { nl_back[2] + 1 } else { i };
    // If the radius cap cut a line, don't start on a zero-width element (e.g. a
    // UTF-8 continuation byte), which would render as a stray glyph.
    while win_start < loc && code[win_start].display_width() == 0 {
        win_start += 1;
    }
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
///
/// Lines are split on the raw `code` elements and each line is formatted on its
/// own. Offsets are never carried over from formatted text back into `code`:
/// `format_slice` may change length (e.g. lossy UTF-8 turns one invalid byte
/// into a 3-byte U+FFFD), so mixing the two coordinate systems is unsound.
pub(crate) fn context_lines<T: Atomic>(code: &[T], loc: usize) -> Vec<String> {
    let len = code.len();
    let loc = loc.min(len);
    // Past the scan threshold we don't resolve line structure, so there is no
    // snippet to render — the byte offset alone is the location.
    let pos = match readable_position(code, loc) {
        Some(pos) => pos,
        None => return Vec::new(),
    };
    let (win_start, win_end, first_line) = context_window(code, loc, pos.line);

    let mut lines = Vec::new();

    // `line_start..line_end` are absolute indices into `code`.
    let mut emit = |line: usize, line_start: usize, line_end: usize| {
        // ±2 lines of context around the error line
        if line + 2 < pos.line || line > pos.line + 2 {
            return;
        }
        let is_err = line == pos.line;
        let prefix = if is_err {
            format!("  > {} | ", line)
        } else {
            format!("    {} | ", line)
        };
        lines.push(format!(
            "{}{}",
            prefix,
            T::format_slice(&code[line_start..line_end])
        ));

        if is_err {
            // Column of the error within the (windowed) line, in display widths.
            let col: usize = code[line_start..loc]
                .iter()
                .map(|e| e.display_width())
                .sum();
            lines.push(format!("{}^--- here", " ".repeat(prefix.len() + col)));
        }
    };

    let mut line = first_line;
    let mut line_start = win_start;
    for (i, element) in code.iter().enumerate().take(win_end).skip(win_start) {
        if element.is_newline() {
            emit(line, line_start, i);
            line += 1;
            line_start = i + 1;
        }
    }

    // Handle the last line if there is no trailing newline (this is also the
    // binary / no-newline case, where the whole window is one "line"). An
    // empty last line is still rendered when it holds the error, e.g. an
    // unexpected EOF after a trailing newline, so the pointer has a home.
    if line_start < win_end || line == pos.line {
        emit(line, line_start, win_end);
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
        let pos = readable_position(code, loc).expect("reference used only on small inputs");
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
                    let pointer_offset = prefix.len() + pos.column;
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
        if line_start < text.len() || current_line == pos.line {
            emit(current_line, line_start, text.len(), &text, &mut lines);
        }
        lines
    }

    #[test]
    fn readable_position_basic() {
        let code = b"abc\ndef\nghi";
        // position of 'e' (index 5) is line 2, column 1
        let p = readable_position(code, 5).unwrap();
        assert_eq!(p.line, 2);
        assert_eq!(p.column, 1);
    }

    #[test]
    fn readable_position_start() {
        let p = readable_position(b"hello", 0).unwrap();
        assert_eq!(p.line, 1);
        assert_eq!(p.column, 0);
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
        assert_eq!(readable_position(code, loc).unwrap().line, 4);
        let out = context_lines(code, loc);
        assert!(out[0].starts_with("    2 | "), "got {:?}", out[0]);
        // error line is line 4, rendered with the `>` marker
        assert!(out.iter().any(|l| l.starts_with("  > 4 | ")), "got {out:?}");
    }

    #[test]
    fn large_binary_below_threshold_is_windowed() {
        // No newlines, but within the scan threshold: the whole thing is
        // "line 1" and rendering must only touch a bounded window around the
        // error, so the output stays tiny relative to the input.
        let size = 64 * CONTEXT_RADIUS; // well under MAX_POSITION_SCAN
        assert!(size <= MAX_POSITION_SCAN);
        let code = vec![0xABu8; size];
        let loc = code.len() - 1;
        let out = context_lines(&code, loc);

        let total: usize = out.iter().map(|l| l.len()).sum();
        // Bounded by ~2*RADIUS of content plus a pointer line, not the input size.
        assert!(
            total <= 8 * CONTEXT_RADIUS,
            "context output not bounded: {total} bytes"
        );
        assert!(out.iter().any(|l| l.starts_with("  > 1 | ")), "got {out:?}");
        assert!(out.iter().any(|l| l.contains("^--- here")), "got {out:?}");
    }

    #[test]
    fn past_scan_threshold_skips_line_resolution() {
        // An error far into a large buffer: we don't scan for line structure.
        // readable_position is None and no context snippet is produced — the
        // byte offset alone is the location.
        let code = vec![0xABu8; MAX_POSITION_SCAN + 16];
        let loc = code.len() - 1;
        assert!(readable_position(&code, loc).is_none());
        assert!(context_lines(&code, loc).is_empty());
    }

    #[test]
    fn early_error_in_huge_buffer_still_resolves() {
        // Gating is on the error position, not buffer size: an error near the
        // start of a huge buffer is cheap and still gets full line/column.
        let mut code = vec![b'x'; MAX_POSITION_SCAN * 4];
        code[3] = b'\n';
        let p = readable_position(&code, 10).expect("early error should resolve");
        assert_eq!(p.line, 2);
        assert!(!context_lines(&code, 10).is_empty());
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
        let p = readable_position(code, 999).unwrap();
        assert_eq!(p.line, 1);
    }

    #[test]
    fn invalid_utf8_before_newline_does_not_panic() {
        // Lossy decoding expands each 0xFF into a 3-byte U+FFFD; offsets in the
        // formatted text must never be used to index `code`.
        let mut code = vec![0xFFu8; 10];
        code.extend_from_slice(b"\nxy");
        let out = context_lines(&code, 12);
        assert!(
            out.iter().any(|l| l.starts_with("  > 2 | xy")),
            "got {out:?}"
        );
        assert!(out.iter().any(|l| l == "         ^--- here"), "got {out:?}");
    }

    #[test]
    fn multibyte_utf8_counts_as_one_column() {
        let code = "let café = ?".as_bytes();
        let loc = code.len() - 1; // the '?'
        assert_eq!(readable_position(code, loc).unwrap().column, 11);
        let out = context_lines(code, loc);
        let line = out.iter().position(|l| l.starts_with("  > ")).unwrap();
        let caret = out[line + 1].find('^').unwrap();
        let qmark = out[line].chars().position(|c| c == '?').unwrap();
        assert_eq!(caret, qmark, "got {out:?}");
    }

    #[test]
    fn window_does_not_start_mid_character() {
        // A long line of 2-byte chars, cut by the radius at an odd offset
        let mut code = "é".repeat(CONTEXT_RADIUS).into_bytes();
        code.push(b'!');
        let loc = code.len() - 1;
        let out = context_lines(&code, loc);
        assert!(!out[0].contains('\u{FFFD}'), "got {out:?}");
    }

    #[test]
    fn eof_after_trailing_newline_has_pointer() {
        let code = b"a\nb\n";
        let out = context_lines(code, code.len());
        assert!(out.iter().any(|l| l.starts_with("  > 3 | ")), "got {out:?}");
        assert!(out.iter().any(|l| l.contains("^--- here")), "got {out:?}");
    }

    #[test]
    fn random_bytes_never_panic() {
        // Deterministic xorshift so failures are reproducible without a dep
        let mut state: u64 = 0x2545F4914F6CDD1D;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let alphabet = [b'a', b'\n', 0xC3, 0xA9, 0xFF, 0x80, 0xE2, 0xF0, b' '];
        for _ in 0..2000 {
            let len = (next() % (3 * CONTEXT_RADIUS as u64)) as usize;
            let code: Vec<u8> = (0..len)
                .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
                .collect();
            let loc = (next() % (len as u64 + 2)) as usize;
            let _ = readable_position(&code, loc);
            let _ = context_lines(&code, loc);
        }
    }
}
