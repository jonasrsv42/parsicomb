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

pub mod ascii;
pub mod atomic;
pub mod byte;
pub mod combinator;
pub mod cursor;
pub mod cursors;
pub mod error;
pub mod parser;
pub mod prelude;
pub mod utf8;

pub use atomic::{
    Atomic, atomic, atomic_between, default, eof, is_atomic, is_sequence, skip_while, skip_while1,
};
pub use combinator::{
    AndExt, AndThenExt, BetweenExt, BoxedExt, BoxedParser, CommitExt, FilterExt, LabelExt, ManyExt,
    MapErrExt, MapExt, MaxDepthExt, NotExt, OptionalExt, OrExt, PositionExt, PrecededExt,
    RecognizeExt, SeparatedListExt, SomeExt, Span, TerminatedExt, all, and, and_then,
    argument_list, between, commit, filter, label, lazy, many, map, map_err, max_depth, not,
    ntimes, optional, or, parser_fn, position, preceded, recognize, separated_list, separated_pair,
    skip_many, skip_some, skip_then, some, take_until, terminated,
};
pub use cursor::Cursor;
pub use cursors::{AtomicCursor, ByteCursor};
pub use error::{
    Advance, CodeLoc, ErrorLeaf, ErrorNode, Expected, ParsicombError, Progress, WithProgress,
};
pub use parser::Parser;
