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

pub mod all;
pub mod and;
pub mod ascii;
pub mod atomic;
pub mod between;
pub mod byte;
pub mod cursor;
pub mod cursors;
pub mod default;
pub mod error;
pub mod filter;
pub mod lazy;
pub mod many;
pub mod map;
pub mod map_err;
pub mod or;
pub mod parser;
pub mod position;
pub mod separated_list;
pub mod separated_pair;
pub mod some;
pub mod take_until;
pub mod utf8;

pub use all::all;
pub use atomic::{Atomic, AtomicParser, atomic};
pub use between::between;
pub use cursor::Cursor;
pub use cursors::{AtomicCursor, ByteCursor};
pub use error::{CodeLoc, ErrorLeaf, ErrorNode, ParsicombError};
pub use lazy::{Lazy, lazy};
pub use parser::Parser;
pub use position::{Position, PositionExt, Span, position};
pub use separated_list::separated_list;
pub use separated_pair::separated_pair;
