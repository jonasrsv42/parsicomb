//! Building parsers: recursion, type erasure, closures

mod boxed;
mod lazy;
mod parser_fn;

pub use boxed::*;
pub use lazy::*;
pub use parser_fn::*;
