//! Skip runs of characters matching a predicate

use crate::ByteCursor;
use crate::ParsicombError;
use crate::cursor::Cursor;
use crate::error::Expected;
use crate::parser::Parser;
use crate::utf8::char;
use std::marker::PhantomData;

/// Advance past characters while `pred` holds; stops before the first
/// character that doesn't match or can't be decoded
#[inline]
fn advance_chars_while<'code>(
    mut cursor: ByteCursor<'code>,
    pred: impl Fn(char) -> bool,
) -> ByteCursor<'code> {
    loop {
        match cursor.value() {
            // ASCII is checked inline; only other characters are decoded
            Ok(byte) if byte.is_ascii() => {
                if !pred(byte as char) {
                    break;
                }
                cursor = cursor.next();
            }
            Ok(_) => match char().parse(cursor) {
                Ok((c, next)) if pred(c) => cursor = next,
                _ => break,
            },
            Err(_) => break,
        }
    }
    cursor
}

/// Parser that skips zero or more UTF-8 characters matching a predicate
///
/// Unlike `skip_many(char().filter(..))`, it builds no error where the run
/// ends; invalid UTF-8 ends the run and isn't reported. Pair it with
/// [`recognize`](crate::combinator::recognize) to get the span it skipped.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct SkipCharsWhile<'code, F> {
    pred: F,
    _input: PhantomData<&'code [u8]>,
}

impl<'code, F: Fn(char) -> bool> Parser for SkipCharsWhile<'code, F> {
    type Cursor = ByteCursor<'code>;
    type Output = ();
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        Ok(((), advance_chars_while(cursor, &self.pred)))
    }
}

/// Create a parser that skips zero or more characters matching `pred`
pub fn skip_chars_while<'code, F: Fn(char) -> bool>(pred: F) -> SkipCharsWhile<'code, F> {
    SkipCharsWhile {
        pred,
        _input: PhantomData,
    }
}

/// Parser that skips one or more UTF-8 characters matching a predicate
///
/// Fails with `expected <label>, found …` when the first character
/// doesn't match; invalid UTF-8 ends the run and isn't reported.
#[must_use = "parsers do nothing until `.parse()` is called"]
pub struct SkipCharsWhile1<'code, F> {
    pred: F,
    label: &'static str,
    _input: PhantomData<&'code [u8]>,
}

impl<'code, F: Fn(char) -> bool> Parser for SkipCharsWhile1<'code, F> {
    type Cursor = ByteCursor<'code>;
    type Output = ();
    type Error = ParsicombError<'code>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let end = advance_chars_while(cursor, &self.pred);
        if end.position() == cursor.position() {
            return Err(cursor.expected_error(Expected::Label(self.label)));
        }
        Ok(((), end))
    }
}

/// Create a parser that skips one or more characters matching `pred`,
/// described by `label` in errors (e.g. "an identifier character")
pub fn skip_chars_while1<'code, F: Fn(char) -> bool>(
    pred: F,
    label: &'static str,
) -> SkipCharsWhile1<'code, F> {
    SkipCharsWhile1 {
        pred,
        label,
        _input: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinator::recognize;

    fn ident(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    #[test]
    fn test_skips_ascii_and_multibyte_characters() {
        let word = recognize(skip_chars_while(ident));
        let (span, _) = word
            .parse(ByteCursor::new("größe_1 rest".as_bytes()))
            .unwrap();
        assert_eq!(span.slice(), "größe_1".as_bytes());
    }

    #[test]
    fn test_stops_before_non_matching_multibyte_character() {
        let ascii_letters = recognize(skip_chars_while(|c| c.is_ascii_alphabetic()));
        let (span, cursor) = ascii_letters
            .parse(ByteCursor::new("abé".as_bytes()))
            .unwrap();
        assert_eq!(span.slice(), b"ab");
        assert_eq!(cursor.position(), 2);
    }

    #[test]
    fn test_stops_before_invalid_utf8() {
        let (_, cursor) = skip_chars_while(|_| true)
            .parse(ByteCursor::new(b"ab\xFFcd"))
            .unwrap();
        assert_eq!(cursor.position(), 2);
    }

    #[test]
    fn test_skip_chars_while1_requires_one_match() {
        let word = skip_chars_while1(ident, "an identifier character");
        assert!(word.parse(ByteCursor::new(b"x+")).is_ok());

        let message = word.parse(ByteCursor::new(b"+")).unwrap_err().to_string();
        assert!(
            message.contains("expected an identifier character, found '+'"),
            "got {message}"
        );
    }
}
