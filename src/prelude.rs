//! The traits needed for method syntax, and the core types
//!
//! ```
//! use parsicomb::prelude::*;
//! use parsicomb::ascii::u64;
//! use parsicomb::byte::is_byte;
//! use parsicomb::eof;
//!
//! let pair = u64().terminated_by(is_byte(b',')).and(u64()).terminated_by(eof());
//! let ((a, b), _) = pair.parse(ByteCursor::new(b"1,2")).unwrap();
//! assert_eq!((a, b), (1, 2));
//! ```

pub use crate::combinator::AndExt;
pub use crate::combinator::AndThenExt;
pub use crate::combinator::AttemptExt;
pub use crate::combinator::BetweenExt;
pub use crate::combinator::BoxedExt;
pub use crate::combinator::CommitExt;
pub use crate::combinator::FilterExt;
pub use crate::combinator::LabelExt;
pub use crate::combinator::ManyExt;
pub use crate::combinator::MapErrExt;
pub use crate::combinator::MapExt;
pub use crate::combinator::MaxDepthExt;
pub use crate::combinator::NotExt;
pub use crate::combinator::OptionalExt;
pub use crate::combinator::OrExt;
pub use crate::combinator::PositionExt;
pub use crate::combinator::PrecededExt;
pub use crate::combinator::RecognizeExt;
pub use crate::combinator::SeparatedListExt;
pub use crate::combinator::SomeExt;
pub use crate::combinator::TerminatedExt;
pub use crate::cursor::Cursor;
pub use crate::cursors::ByteCursor;
pub use crate::error::{ErrorNode, ParsicombError};
pub use crate::parser::Parser;
