//! ASCII numbers

use crate::ByteCursor;
use crate::ParsicombError;
use crate::atomic::skip_while1;
use crate::byte::is_byte;
use crate::cursor::Cursor;
use crate::map_err::MapErrExt;
use crate::optional::optional;
use crate::or::OrExt;
use crate::parser::Parser;
use crate::parser_fn::parser_fn;
use crate::recognize::recognize;

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
/// "number too large" for an integer that does not fit in an i64. The input is
/// scanned once.
pub fn number<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = Number, Error = ParsicombError<'code>> {
    parser_fn(|start: ByteCursor<'code>| {
        let (sign, after_sign) = optional(sign()).parse(start)?;
        let (integer, integer_end) = recognize(digits()).parse(after_sign)?;

        if let Some(fraction_start) = fraction_start(integer_end) {
            let (_, end) = digits().parse(fraction_start)?;
            return Ok((Number::F64(float_value(start, end)?), end));
        }

        let magnitude = unsigned_value(integer_end, integer.slice())?;
        Ok((
            Number::I64(signed_value(integer_end, sign, magnitude)?),
            integer_end,
        ))
    })
}

/// Parser for a leading `-` or `+` sign
fn sign<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = u8, Error = ParsicombError<'code>> {
    is_byte(b'-')
        .or(is_byte(b'+'))
        .map_err(ParsicombError::from)
}

/// One or more ASCII digits; fails with `expected a digit` if there are none,
/// and builds no error where a run of digits ends
fn digits<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = (), Error = ParsicombError<'code>> {
    skip_while1(|b: u8| b.is_ascii_digit(), "a digit")
}

/// The cursor after the `.` if `cursor` is at `.` followed by a digit
fn fraction_start(cursor: ByteCursor<'_>) -> Option<ByteCursor<'_>> {
    let after_dot = cursor.next();
    let is_fraction = cursor.value().is_ok_and(|b| b == b'.')
        && after_dot.value().is_ok_and(|b| b.is_ascii_digit());
    is_fraction.then_some(after_dot)
}

/// The value of a run of ASCII digits; overflow is reported at `end`, after
/// the digits
fn unsigned_value<'code>(
    end: ByteCursor<'code>,
    digits: &[u8],
) -> Result<u64, ParsicombError<'code>> {
    digits
        .iter()
        .try_fold(0u64, |value, digit| {
            value.checked_mul(10)?.checked_add(u64::from(digit - b'0'))
        })
        .ok_or_else(|| end.invalid_error(too_large(digits).into()))
}

/// `number too large: <digits>`, with long runs of digits shortened so the
/// message stays small however long the input number is
fn too_large(digits: &[u8]) -> String {
    const SHOWN: usize = 24;
    let shown = String::from_utf8_lossy(digits.get(..SHOWN).unwrap_or(digits));
    if digits.len() > SHOWN {
        format!("number too large: {shown}… ({} digits)", digits.len())
    } else {
        format!("number too large: {shown}")
    }
}

/// `magnitude` with `sign` applied; overflow is reported at `end`, after the
/// number
fn signed_value<'code>(
    end: ByteCursor<'code>,
    sign: Option<u8>,
    magnitude: u64,
) -> Result<i64, ParsicombError<'code>> {
    if sign == Some(b'-') {
        // Subtracting from 0 reaches i64::MIN, which has no positive counterpart
        0i64.checked_sub_unsigned(magnitude).ok_or_else(|| {
            end.invalid_error(format!("negative number too large: -{}", magnitude).into())
        })
    } else {
        i64::try_from(magnitude).map_err(|_| {
            end.invalid_error(format!("positive number too large: {}", magnitude).into())
        })
    }
}

/// The float in `start..end`, which holds `[sign]digits.digits`; overflow is
/// reported at `end`
fn float_value<'code>(
    start: ByteCursor<'code>,
    end: ByteCursor<'code>,
) -> Result<f64, ParsicombError<'code>> {
    // The standard library does the correctly rounded conversion. The matched
    // slice is ASCII, so the conversions cannot fail in practice; map them to
    // errors anyway rather than panic.
    let span = start.span_to(end);
    let value: f64 = std::str::from_utf8(span.slice())
        .ok()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| end.invalid_error("invalid float literal".into()))?;
    if !value.is_finite() {
        return Err(end.invalid_error("floating point overflow".into()));
    }
    Ok(value)
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
