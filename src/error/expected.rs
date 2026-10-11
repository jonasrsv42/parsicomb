use super::CodeLoc;
use crate::atomic::Atomic;
use std::borrow::Cow;
use std::fmt;

/// What a leaf parser expected to find
///
/// Holds only the expectation; what was actually found is read from the input
/// at the error location when the error is displayed. Building one does not
/// allocate, except cloning a `Sequence` that owns its elements.
#[derive(Debug, Clone)]
pub enum Expected<T: Atomic> {
    /// A specific element
    Element(T),
    /// An element in an inclusive range
    Range(T, T),
    /// An element described by a label, e.g. "a non-reserved token"
    Label(&'static str),
    /// A run of elements, of which the first `matched` matched before the
    /// element at the error location
    Sequence {
        expected: Cow<'static, [T]>,
        matched: usize,
    },
}

/// Formats an element through [`Atomic::describe`]
struct Describe<'a, T>(&'a T);

impl<T: Atomic> fmt::Display for Describe<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.describe(f)
    }
}

impl<T: Atomic> Expected<T> {
    /// Write `expected X, found Y`, reading what was found from the input at `loc`
    pub(super) fn write_message(
        &self,
        loc: &CodeLoc<'_, T>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let code = loc.code();
        let at = loc.position();

        match self {
            Expected::Element(expected) => write!(f, "expected {}, ", Describe(expected))?,
            Expected::Range(start, end) => {
                write!(f, "expected {} to {}, ", Describe(start), Describe(end))?
            }
            Expected::Label(label) => write!(f, "expected {}, ", label)?,
            Expected::Sequence { expected, matched } => {
                write!(f, "expected {:?}, ", T::format_slice(expected))?;

                // Found: from the start of the sequence through the mismatching
                // element, plus any zero-width elements completing it (e.g. the
                // rest of a multi-byte character)
                let start = at.saturating_sub(*matched).min(code.len());
                let mut end = (at + 1).min(code.len());
                while end < code.len() && code[end].display_width() == 0 {
                    end += 1;
                }
                return if start == end {
                    write!(f, "found end of input")
                } else if at >= code.len() {
                    write!(
                        f,
                        "found {:?} at end of input",
                        T::format_slice(&code[start..end])
                    )
                } else {
                    write!(f, "found {:?}", T::format_slice(&code[start..end]))
                };
            }
        }

        match code.get(at) {
            Some(element) => write!(f, "found {}", Describe(element)),
            None => write!(f, "found end of input"),
        }
    }
}
