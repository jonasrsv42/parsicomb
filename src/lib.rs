//! # ParsiComb - Parser Combinator Library
//!
//! A parser combinator library for areamy, designed for building the mao/lang parser.
//! 
//! ParsiComb provides composable, type-safe parsers that can be combined to build
//! complex parsing logic from simple building blocks. The library emphasizes:
//! 
//! - **Zero panics**: All parsing errors are handled through `Result` types
//! - **Rich error reporting**: Provides line numbers, context, and detailed error messages
//! - **Composability**: Small parsers combine into larger ones using combinators
//! - **Performance**: Efficient byte-level parsing with minimal allocations

pub mod byte;
pub mod byte_cursor;
pub mod error;
pub mod parser;
pub mod many;
pub mod some;
pub mod ascii;
pub mod utf8;
pub mod or;
pub mod map;
pub mod and;
pub mod filter;
pub mod take_until;
pub mod default;

pub use error::{CodeLoc, ParsiCombError};
pub use parser::Parser;
