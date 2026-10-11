//! ASCII numbers

use crate::ByteCursor;
use crate::ParsicombError;
use crate::and::AndExt;
use crate::byte::is_byte;
use crate::map::MapExt;
use crate::optional::optional;
use crate::or::OrExt;
use crate::parser::Parser;
use crate::parser_fn::parser_fn;
use crate::skip_some::skip_some;

pub mod digit;
pub mod f64;
pub mod i64;
pub mod u64;

pub use digit::digit;
pub use f64::f64;
pub use i64::i64;
pub use u64::u64;

/// A parsed number literal
#[derive(Debug, PartialEq)]
pub enum Number {
    /// An integer, from [`i64()`]
    I64(i64),
    /// A number with a fraction, from [`f64()`]
    F64(f64),
}

/// Parser that matches an integer or a float and returns a [`Number`]
///
/// A number is a float when its digits are followed by `.` and a digit, and
/// an integer otherwise; it then fails as [`f64()`] or [`i64()`] would, e.g. with
/// "number too large" for an integer that does not fit in an i64.
pub fn number<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = Number, Error = ParsicombError<'code>> {
    parser_fn(|start: ByteCursor<'code>| {
        let (_, integer_end) = optional(sign()).and(skip_some(digit())).parse(start)?;
        if is_byte(b'.').and(digit()).parse(integer_end).is_ok() {
            f64().map(Number::F64).parse(start)
        } else {
            i64().map(Number::I64).parse(start)
        }
    })
}

/// Parser for a leading `-` or `+` sign
fn sign<'code>() -> impl Parser<Cursor = ByteCursor<'code>, Output = u8> {
    is_byte(b'-').or(is_byte(b'+'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;

    #[test]
    fn test_number_float() {
        let data = b"2.5abc";
        let cursor = ByteCursor::new(data);
        let parser = number();

        let (num, cursor) = parser.parse(cursor).unwrap();
        match num {
            Number::F64(f) => assert!((f - 2.5).abs() < f64::EPSILON),
            Number::I64(_) => panic!("Expected float, got int"),
        }
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_number_int() {
        let data = b"123abc";
        let cursor = ByteCursor::new(data);
        let parser = number();

        let (num, cursor) = parser.parse(cursor).unwrap();
        match num {
            Number::I64(i) => assert_eq!(i, 123),
            Number::F64(_) => panic!("Expected int, got float"),
        }
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_number_negative_float() {
        let data = b"-2.5xyz";
        let cursor = ByteCursor::new(data);
        let parser = number();

        let (num, cursor) = parser.parse(cursor).unwrap();
        match num {
            Number::F64(f) => assert!((f - (-2.5)).abs() < f64::EPSILON),
            Number::I64(_) => panic!("Expected float, got int"),
        }
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_number_negative_int() {
        let data = b"-456xyz";
        let cursor = ByteCursor::new(data);
        let parser = number();

        let (num, cursor) = parser.parse(cursor).unwrap();
        match num {
            Number::I64(i) => assert_eq!(i, -456),
            Number::F64(_) => panic!("Expected int, got float"),
        }
        assert_eq!(cursor.value().unwrap(), b'x');
    }

    #[test]
    fn test_number_integer_overflow_says_too_large() {
        for input in [
            &b"99999999999999999999999"[..],
            b"-99999999999999999999999",
            b"9223372036854775808",
        ] {
            let message = number()
                .parse(ByteCursor::new(input))
                .unwrap_err()
                .to_string();
            assert!(message.contains("too large"), "got {message}");
        }
    }

    #[test]
    fn test_number_dot_without_fraction_is_an_integer() {
        let (num, cursor) = number().parse(ByteCursor::new(b"1.x")).unwrap();
        assert_eq!(num, Number::I64(1));
        assert_eq!(cursor.position(), 1);
    }
}
