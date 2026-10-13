//! Combinators: parsers built from other parsers
//!
//! Every combinator's parser type, error type, function and method trait,
//! in one flat namespace. The functions and method traits are also at the
//! crate root, and the method traits in the [`prelude`](crate::prelude).
//!
//! # Controlling backtracking
//!
//! By default [`or`] tries every alternative, and [`optional`] and the
//! repetitions stop only where their parser matched nothing; a failure after
//! matching input is passed on. Where that isn't what a grammar needs:
//!
//! - [`commit`] makes failures final: `or` stops trying alternatives, and
//!   `optional` and the repetitions pass them on.
//! - [`attempt`] makes failures count as nothing matched (Parsec's `try`):
//!   `optional` and the repetitions backtrack from them.
//! - [`not`] is negative lookahead: it succeeds where its parser fails,
//!   without consuming input.
//! - [`max_depth`] bounds recursion with a committed failure.
//!
//! The [`error`](crate::error) module docs describe the model in full.
//!
//! A glob import of this module shadows [`Option::Some`] with the [`Some`]
//! combinator; import names from it individually.

mod build;
mod choice;
mod control;
mod repeat;
mod sequence;
mod transform;

pub use build::*;
pub use choice::*;
pub use control::*;
pub use repeat::*;
pub use sequence::*;
pub use transform::*;
