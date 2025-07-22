use crate::map::MapExt;
use crate::map_err::MapErrExt;
use crate::or::OrExt;
use crate::parser::Parser;
use crate::ParsicombError;

pub mod digit;
pub mod f64;
pub mod i64;
pub mod u64;

pub use digit::digit;
pub use f64::f64;
pub use i64::i64;
pub use u64::u64;

#[derive(Debug, PartialEq)]
pub enum Number {
    I64(i64),
    F64(f64),
}

/// Parser that matches either an integer or a float and returns a Number enum
pub fn number<'code>() -> impl Parser<'code, Output = Number, Error = ParsicombError<'code>> {
    f64().map(Number::F64).or(i64().map(Number::I64))
        .map_err(|or_err| or_err.furthest())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_cursor::ByteCursor;

    #[test]
    fn test_number_float() {
        let data = b"3.14abc";
        let cursor = ByteCursor::new(data);
        let parser = number();

        let (num, cursor) = parser.parse(cursor).unwrap();
        match num {
            Number::F64(f) => assert!((f - 3.14).abs() < f64::EPSILON),
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
}
