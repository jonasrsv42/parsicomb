//! How far a failed parser advanced past where it started

/// How far a failed parser advanced past `start`, where whoever ran it
/// started it (see [`Progress::since`](super::Progress::since))
///
/// Combinators that can recover from a failure, by trying something else or
/// stopping, decide by this: the further a failed parser advanced, the fewer
/// combinators backtrack from it. [How failures are
/// handled](crate#how-failures-are-handled) lists what each built-in
/// combinator does.
///
/// The cases are ordered: `None < Partial < Committed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Advance {
    /// No progress past the start: what was expected isn't there
    None,
    /// Progress past the start, then a failure: something here is wrong
    Partial,
    /// Committed to this path before failing, or failed fatally: nothing
    /// backtracks
    Committed,
}
