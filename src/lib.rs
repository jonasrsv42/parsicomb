#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![forbid(unsafe_code)]
// Parsers run on untrusted input: no panics, and no casts that silently lose
// or change values. Tests may panic (see clippy.toml).
#![deny(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::panic_in_result_fn,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::dbg_macro,
    clippy::print_stdout,
    clippy::print_stderr
)]
// Test inputs are built with plain casts
#![cfg_attr(
    test,
    allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )
)]

pub mod all;
pub mod and;
pub mod and_then;
pub mod argument_list;
pub mod ascii;
pub mod atomic;
pub mod between;
pub mod boxed;
pub mod byte;
pub mod commit;
pub mod cursor;
pub mod cursors;
pub mod default;
pub mod error;
pub mod filter;
pub mod label;
pub mod lazy;
pub mod many;
pub mod map;
pub mod map_err;
pub mod max_depth;
pub mod not;
pub mod ntimes;
pub mod optional;
pub mod or;
pub mod parser;
pub mod parser_fn;
pub mod position;
pub mod recognize;
pub mod separated_list;
pub mod separated_pair;
pub mod skip_many;
pub mod skip_some;
pub mod skip_then;
pub mod some;
pub mod take_until;
pub mod utf8;

pub use all::all;
pub use and::{AndExt, and};
pub use and_then::{AndThenExt, and_then};
pub use argument_list::argument_list;
pub use atomic::{Atomic, atomic, atomic_between, is_atomic, is_sequence, skip_while, skip_while1};
pub use between::between;
pub use boxed::{BoxedExt, BoxedParser};
pub use commit::{CommitExt, commit};
pub use cursor::Cursor;
pub use cursors::{AtomicCursor, ByteCursor};
pub use default::default;
pub use error::{Advance, CodeLoc, ErrorLeaf, ErrorNode, Expected, ParsicombError, Progress};
pub use filter::{FilterExt, filter};
pub use label::{LabelExt, label};
pub use lazy::lazy;
pub use many::many;
pub use map::{MapExt, map};
pub use map_err::{MapErrExt, map_err};
pub use max_depth::{MaxDepthExt, max_depth};
pub use not::{NotExt, not};
pub use ntimes::ntimes;
pub use optional::optional;
pub use or::{OrExt, or};
pub use parser::Parser;
pub use parser_fn::parser_fn;
pub use position::{PositionExt, Span, position};
pub use recognize::{RecognizeExt, recognize};
pub use separated_list::separated_list;
pub use separated_pair::separated_pair;
pub use skip_many::skip_many;
pub use skip_some::skip_some;
pub use skip_then::skip_then;
pub use some::some;
pub use take_until::take_until;
