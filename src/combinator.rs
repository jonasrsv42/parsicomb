//! Combinators: parsers built from other parsers
//!
//! Every combinator's parser type, error type, function and method trait,
//! in one flat namespace. The functions and method traits are also at the
//! crate root, and the method traits in the [`prelude`](crate::prelude).
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
