//! Error positions and messages for a JSON grammar written the obvious way:
//! `optional(separated_list(..))` for lists, whitespace after each token, and
//! no `commit` in arrays and objects

use parsicomb::ascii::{Number, number};
use parsicomb::byte::{byte, is_byte};
use parsicomb::utf8::is_string;
use parsicomb::{
    AndExt, BoxedExt, ByteCursor, CommitExt, Cursor, LabelExt, MapErrExt, MapExt, OrExt, Parser,
    ParsicombError, lazy, optional, recognize, separated_list, skip_many, skip_while, skip_while1,
};

#[derive(Debug)]
#[allow(dead_code)]
enum Json<'c> {
    Null,
    Bool(bool),
    Num(Number),
    Str(&'c [u8]),
    Arr(Vec<Json<'c>>),
    Obj(Vec<(&'c [u8], Json<'c>)>),
}

type E<'c> = ParsicombError<'c>;

fn ws<'c>() -> impl Parser<Cursor = ByteCursor<'c>, Output = (), Error = E<'c>> {
    skip_while(|b: u8| matches!(b, b' ' | b'\n' | b'\t' | b'\r'))
}

fn tok<'c>(b: u8) -> impl Parser<Cursor = ByteCursor<'c>, Output = u8, Error = E<'c>> {
    is_byte(b).and(ws()).map(|(x, _)| x).map_err(E::from)
}

fn string<'c>() -> impl Parser<Cursor = ByteCursor<'c>, Output = &'c [u8], Error = E<'c>> {
    let plain = skip_while1(|b: u8| b != b'"' && b != b'\\', "a string character");
    let escape = is_byte(b'\\').and(byte()).map(|_| ());
    let body = recognize(skip_many(plain.or(escape)));
    is_byte(b'"')
        .and(body.and(is_byte(b'"')).commit())
        .and(ws())
        .map(|((_, (span, _)), _)| span.slice())
        .map_err(E::from)
}

fn value<'c>() -> impl Parser<Cursor = ByteCursor<'c>, Output = Json<'c>, Error = E<'c>> {
    let rec = || lazy(|| value().boxed());
    let array = tok(b'[')
        .and(optional(separated_list(rec(), tok(b','))).and(tok(b']')))
        .map(|(_, (items, _))| Json::Arr(items.unwrap_or_default()));
    let member = string().and(tok(b':')).and(rec()).map(|((k, _), v)| (k, v));
    let object = tok(b'{')
        .and(optional(separated_list(member, tok(b','))).and(tok(b'}')))
        .map(|(_, (items, _))| Json::Obj(items.unwrap_or_default()));
    let num = number().and(ws()).map(|(n, _)| Json::Num(n));
    let kw = |s: &'static str, v: fn() -> Json<'c>| is_string(s).and(ws()).map(move |_| v());
    string()
        .map(Json::Str)
        .or(num)
        .or(object)
        .or(array)
        .or(kw("true", || Json::Bool(true)))
        .or(kw("false", || Json::Bool(false)))
        .or(kw("null", || Json::Null))
        .label("a JSON value")
        .map_err(E::from)
}

fn first_line(input: &[u8]) -> String {
    match ws().and(value()).parse(ByteCursor::new(input)) {
        Ok((_, rest)) if rest.eos() => "OK".into(),
        Ok((_, rest)) => format!("trailing input at {}", rest.position()),
        Err(error) => error.to_string().lines().next().unwrap().to_string(),
    }
}

#[test]
fn malformed_json_is_reported_where_it_goes_wrong() {
    let cases: [(&[u8], &str); 11] = [
        (
            br#"{"a": 1, "b": [1, 2,, 3]}"#,
            "column 21 (offset 20): expected a JSON value, found ','",
        ),
        (
            br#"{"a": 1 "b": 2}"#,
            "column 9 (offset 8): expected '}', found '\"'",
        ),
        (
            br#"{"a": tru}"#,
            r#"column 10 (offset 9): expected "true", found "tru}""#,
        ),
        (
            br#"{"a": "unterminated}"#,
            "column 21 (offset 20): expected '\"', found end of input",
        ),
        (
            b"[1, 2, 3",
            "column 9 (offset 8): expected ']', found end of input",
        ),
        (
            br#"{"key" 1}"#,
            "column 8 (offset 7): expected ':', found '1'",
        ),
        (
            b"",
            "column 1 (offset 0): expected a JSON value, found end of input",
        ),
        (
            b"   \n\n  @",
            "line 3, column 3 (offset 7): expected a JSON value, found '@'",
        ),
        (b"[1.5e10]", "column 5 (offset 4): expected ']', found 'e'"),
        (
            b"[99999999999999999999]",
            "column 22 (offset 21): number too large: 99999999999999999999",
        ),
        (
            b"{\"a\":\n  {\"b\":\n    [1,\n     2\n     3]}}",
            "line 5, column 6 (offset 34): expected ']', found '3'",
        ),
    ];
    for (input, expected) in cases {
        let message = first_line(input);
        assert!(
            message.contains(expected),
            "input {:?}\n  got: {message}\n want: {expected}",
            String::from_utf8_lossy(input)
        );
    }
}

#[test]
fn valid_json_parses() {
    for input in [
        &b"[]"[..],
        b"{}",
        b"[[], {}]",
        br#"{"a": [1, 2.5, "x", true, null]}"#,
    ] {
        assert_eq!(
            first_line(input),
            "OK",
            "input {:?}",
            String::from_utf8_lossy(input)
        );
    }
}
