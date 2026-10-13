# ParsiComb

Parser combinators for Rust: build parsers from small pieces, over bytes, UTF-8
text or your own tokens, and get error messages that point at the problem.

```rust
use parsicomb::prelude::*;
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;

// A list of numbers such as `[1,2,3]`
let list = u64().separated_by(is_byte(b',')).between(is_byte(b'['), is_byte(b']'));
assert_eq!(list.parse(ByteCursor::new(b"[1,2,3]")).unwrap().0, vec![1, 2, 3]);

let error = list.parse(ByteCursor::new(b"[1,2;3]")).unwrap_err();
println!("{error}");
```

prints

```text
Syntax error at line 1, column 5 (offset 4): expected ']', found ';'

  > 1 | [1,2;3]
            ^--- here
```

- **Composable:** `and`, `or`, `many`, `between`, … as functions or methods.
- **Borrowed input:** cursors, spans and errors point into the input instead
  of copying it, and messages are only formatted when an error is displayed.
- **Statically dispatched:** combinators are plain generic types that inline
  into one function; type erasure with `.boxed()` is opt-in.
- **Backtracking you can tune:** errors point at the problem by default;
  `commit` makes failures final, and `attempt` backtracks out of a partial
  match.
- **No panics while parsing:** failures are `Result`s. Bound recursion with
  `max_depth` on untrusted input.

## Contents

- [Install](#install)
- [Your first parser: JSON](#your-first-parser-json)
- [API overview](#api-overview)
- [Recursive grammars](#recursive-grammars)
- [Recipes](#recipes)
- [Error messages](#error-messages)
- [Controlling backtracking](#controlling-backtracking)
- [Untrusted input](#untrusted-input)
- [Hand-written parsers](#hand-written-parsers)
- [Custom element types](#custom-element-types)
- [Performance and compile times](#performance-and-compile-times)

## Install

ParsiComb isn't on crates.io yet. Depend on it from GitHub:

```toml
[dependencies]
parsicomb = { git = "https://github.com/jonasrsv42/parsicomb", branch = "main" }
```

`Cargo.lock` pins the commit you first built with, so upgrades are deliberate
(the API can still change): `cargo update -p parsicomb` moves to the latest.
To pin a commit in `Cargo.toml` instead, use `rev = "<commit>"`.

It needs Rust 1.85 or newer (edition 2024). For the API docs, run
`cargo doc -p parsicomb --open`.

## Your first parser: JSON

A complete JSON parser (without string escapes), parsing a `&str` and
printing the errors:

```rust
use parsicomb::prelude::*;
use parsicomb::ascii::{Number, number};
use parsicomb::byte::is_byte;
use parsicomb::utf8::is_string;
use parsicomb::{eof, lazy, separated_pair, skip_while};

#[derive(Debug)]
#[allow(dead_code)] // the example only prints values
enum Json {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

type Error<'a> = ParsicombError<'a>;

// Whitespace is skipped after every token, so each token starts at its first character
fn ws<'a>() -> impl Parser<Cursor = ByteCursor<'a>, Output = (), Error = Error<'a>> {
    skip_while(|b: u8| b.is_ascii_whitespace())
}

fn token<'a>(b: u8) -> impl Parser<Cursor = ByteCursor<'a>, Output = u8, Error = Error<'a>> {
    is_byte(b).terminated_by(ws()).map_err(Error::from)
}

fn string<'a>() -> impl Parser<Cursor = ByteCursor<'a>, Output = String, Error = Error<'a>> {
    skip_while(|b: u8| b != b'"')
        .recognize()
        .between(is_byte(b'"'), token(b'"'))
        .map(|span| span.as_string())
        .map_err(Error::from)
}

fn value<'a>() -> impl Parser<Cursor = ByteCursor<'a>, Output = Json, Error = Error<'a>> {
    // `value` contains itself: `lazy` builds it on first use, `boxed` names its type
    let value = || lazy(|| value().boxed());

    let keyword = |word: &'static str, json: fn() -> Json| {
        is_string(word).terminated_by(ws()).map(move |_| json())
    };
    let array = value()
        .separated_by(token(b','))
        .optional()
        .between(token(b'['), token(b']'))
        .map(|items| Json::Array(items.unwrap_or_default()));
    let member = separated_pair(string(), token(b':'), value());
    let object = member
        .separated_by(token(b','))
        .optional()
        .between(token(b'{'), token(b'}'))
        .map(|members| Json::Object(members.unwrap_or_default()));

    keyword("null", || Json::Null)
        .or(keyword("true", || Json::Bool(true)))
        .or(keyword("false", || Json::Bool(false)))
        .or(number().terminated_by(ws()).map(Json::Number))
        .or(string().map(Json::String))
        .or(array)
        .or(object)
        .label("a JSON value")
        .map_err(Error::from)
}

// The whole input, and nothing after it
fn parse_json(input: &str) -> Result<Json, Error<'_>> {
    let document = value().preceded_by(ws()).terminated_by(eof());
    let (json, _) = document.parse(ByteCursor::new(input.as_bytes()))?;
    Ok(json)
}

fn main() {
    println!("{:?}", parse_json(r#"{"name": "parsicomb", "tags": [1, 2.5, true, null]}"#));

    for input in [r#"{"tags": [1, 2,, 3]}"#, "[1, 2] 3"] {
        if let Err(error) = parse_json(input) {
            println!("{error}");
        }
    }
}
```

It prints:

```text
Ok(Object([("name", String("parsicomb")), ("tags", Array([Number(I64(1)), Number(F64(2.5)), Bool(true), Null]))]))
Syntax error at line 1, column 16 (offset 15): expected a JSON value, found ','

  > 1 | {"tags": [1, 2,, 3]}
                       ^--- here

Syntax error at line 1, column 8 (offset 7): expected end of input, found '3'

  > 1 | [1, 2] 3
               ^--- here
```

Things to notice:

- A `&str` is parsed as its bytes: `ByteCursor::new(input.as_bytes())`. The
  `utf8` parsers decode characters from them.
- A parser succeeds as soon as it has matched, even with input left over;
  `.terminated_by(eof())` requires the whole input.
- `label` names a set of alternatives: when none of them matches, the error
  says `expected a JSON value` instead of naming the first alternative.
- Each rule returns `impl Parser`, and `.map_err(Error::from)` gives them all
  one concrete error type, which recursion needs (see
  [Recursive grammars](#recursive-grammars)).

## API overview

`use parsicomb::prelude::*` brings in the traits for method syntax (`.and()`,
`.or()`, `.map()`, …) and the core types: `Parser`, `Cursor`, `ByteCursor`,
`ParsicombError` and `ErrorNode`. Everything else is in a few modules:

| Module | What's in it | For example |
|--------|--------------|-------------|
| `byte` | single bytes | `is_byte(b'[')`, `between_bytes(b'a', b'z')` |
| `utf8` | characters and strings | `is_string("let")`, `unicode_letter()` |
| `ascii` | numbers and whitespace | `u64()`, `number()`, `whitespace()` |
| `atomic` | the same for any element type, runs, end of input | `is_atomic(token)`, `skip_while(pred)`, `eof()` |
| `combinator` | parsers built from parsers | `many`, `or`, `between`, `label`, `commit`, `with_position` |

The `atomic` and `combinator` functions are also at the crate root
(`parsicomb::{many, eof, …}`).
Combinators that take a parser first also come as methods, so
`many(p)` can be written `p.many()` and `between(open, p, close)` can be written
`p.between(open, close)`. A parser can be used in several places by
reference: `item.and(&separator)`.

Parsers that match input without collecting it, such as `recognize` and
`is_sequence`, output a `Span`: `span.slice()` gives the matched elements,
`span.as_string()` an owned `String` (replacing invalid UTF-8), and
`span.start()`/`span.end()` the offsets. For source positions on AST nodes,
`p.with_position()` outputs `(value, span)`.

For the full list, see the API docs (`cargo doc -p parsicomb --open`).

## Recursive grammars

A parser that refers to itself goes through `lazy`, which builds the inner
parser on first use. Its type must be erased with `.boxed()`, and its error
type must be concrete, since both would otherwise contain themselves:

- `map_err(ParsicombError::from)` makes the error concrete. A combinator's
  error converts into the `ParsicombError` of whichever part failed,
  allocating only for failures under `commit`, `attempt` or `skip_then`.
- Errors of other types, such as your own error from a `map_err`, can be
  boxed with `ParsicombError::wrap`, which allocates once per failure.

```rust
use parsicomb::prelude::*;
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;
use parsicomb::lazy;

#[derive(Debug, PartialEq)]
enum Value {
    Number(u64),
    List(Vec<Value>),
}

fn value<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = Value, Error = ParsicombError<'code>> {
    let number = u64().map(Value::Number);
    let list = lazy(|| value().boxed())
        .separated_by(is_byte(b','))
        .between(is_byte(b'['), is_byte(b']'))
        .map(Value::List);

    number.or(list).map_err(ParsicombError::from)
}

let (parsed, _) = value().parse(ByteCursor::new(b"[1,[2,3]]")).unwrap();
assert_eq!(
    parsed,
    Value::List(vec![
        Value::Number(1),
        Value::List(vec![Value::Number(2), Value::Number(3)]),
    ])
);
```

The `'code` lifetime ties a built parser to one input, so call `value()`
again for each input. Building a parser is cheap: it allocates only for
`boxed`. Parsers that contain `boxed` or `lazy` are not `Send` or `Sync`, so
each thread builds its own.

## Recipes

### Whitespace and keywords

Skip whitespace *after* tokens, as the JSON parser does: a missing token then
matches nothing, and the alternatives around it get to try. To skip *before*
tokens, use `skip_then(whitespace, token)`, which doesn't count the
whitespace as a match.

Match keywords as whole words: read the word, then check it. If you used
`is_string("let")` instead, it would match the start of `letter`:

```rust
use parsicomb::prelude::*;
use parsicomb::{Span, recognize, skip_while1};

// Letters, digits and `_`, as one span
fn word<'a>() -> impl Parser<Cursor = ByteCursor<'a>, Output = Span<'a>, Error = ParsicombError<'a>> {
    recognize(skip_while1(|b: u8| b.is_ascii_alphanumeric() || b == b'_', "a word"))
}

let keyword = |kw: &'static str| word().filter(move |w| w.slice() == kw.as_bytes(), kw);
let identifier = word().filter(|w| !matches!(w.slice(), b"let" | b"fn"), "an identifier");

assert!(keyword("let").parse(ByteCursor::new(b"let x")).is_ok());
assert!(keyword("let").parse(ByteCursor::new(b"letter")).is_err());
assert!(identifier.parse(ByteCursor::new(b"letter")).is_ok());
assert!(identifier.parse(ByteCursor::new(b"let")).is_err());
```

### Operator precedence

Write one rule per precedence level, each a `many` of the operators of its
level, folded left to right. The tightest level is parsed innermost:

```rust
use parsicomb::prelude::*;
use parsicomb::ascii::i64;
use parsicomb::byte::is_byte;
use parsicomb::{lazy, skip_while};

type Error<'a> = ParsicombError<'a>;

fn token<'a>(b: u8) -> impl Parser<Cursor = ByteCursor<'a>, Output = u8, Error = Error<'a>> {
    is_byte(b).terminated_by(skip_while(|b: u8| b == b' ')).map_err(Error::from)
}

// atom := integer | '(' expr ')'
fn atom<'a>() -> impl Parser<Cursor = ByteCursor<'a>, Output = i64, Error = Error<'a>> {
    let integer = i64().terminated_by(skip_while(|b: u8| b == b' '));
    let parens = lazy(|| expr().boxed()).between(token(b'('), token(b')'));
    integer.or(parens).map_err(Error::from)
}

// term := atom ('*' atom)*
fn term<'a>() -> impl Parser<Cursor = ByteCursor<'a>, Output = i64, Error = Error<'a>> {
    atom()
        .and(token(b'*').and(atom()).many())
        .map(|(first, rest)| rest.into_iter().fold(first, |acc, (_, x)| acc.saturating_mul(x)))
        .map_err(Error::from)
}

// expr := term (('+' | '-') term)*
fn expr<'a>() -> impl Parser<Cursor = ByteCursor<'a>, Output = i64, Error = Error<'a>> {
    let op = token(b'+').or(token(b'-'));
    term()
        .and(op.and(term()).many())
        .map(|(first, rest)| {
            rest.into_iter().fold(first, |acc, (op, x)| match op {
                b'+' => acc.saturating_add(x),
                _ => acc.saturating_sub(x),
            })
        })
        .map_err(Error::from)
}

assert_eq!(expr().parse(ByteCursor::new(b"1 + 2 * (3 - 1) - 4")).unwrap().0, 1);
let error = expr().parse(ByteCursor::new(b"1 + * 2")).unwrap_err();
assert!(error.to_string().contains("expected a digit, found '*'"));
```

### Runs without collecting

`many` and `some` collect their outputs into a `Vec`. When only the matched
input matters, `skip_many`/`skip_some` repeat without collecting, and
`recognize` outputs the span a parser matched. For runs of single elements
or characters, `skip_while` (any element type) and `utf8::skip_chars_while`
check each one with a predicate. Unlike `skip_many(..filter(..))`, they build
no error where the run ends, which matters in hot loops such as lexers:

```rust
use parsicomb::prelude::*;
use parsicomb::recognize;
use parsicomb::utf8::{skip_chars_while, skip_chars_while1};

let word = recognize(skip_chars_while1(char::is_alphanumeric, "a letter or digit"));
let (span, rest) = word.parse(ByteCursor::new("größe 42".as_bytes())).unwrap();
assert_eq!(span.slice(), "größe".as_bytes());

let (_, rest) = skip_chars_while(char::is_whitespace).parse(rest).unwrap();
assert_eq!(word.parse(rest).unwrap().0.slice(), b"42");
```

### Context-dependent parsing

`and_then` runs a parser, then a parser built from its output. Here the
closing quote must repeat as many `#` as opened it, as in Rust raw strings:

```rust
use parsicomb::prelude::*;
use parsicomb::byte::{byte, is_byte};
use parsicomb::{many, ntimes, recognize, skip_many};

let raw = many(is_byte(b'#'))
    .map(|hashes| hashes.len())
    .and_then(|n| {
        recognize(skip_many(byte().filter(|b| *b != b'"', "text")))
            .between(is_byte(b'"'), is_byte(b'"'))
            .terminated_by(ntimes(is_byte(b'#'), n))
    });

let (text, _) = raw.parse(ByteCursor::new(b"##\"hi\"##")).unwrap();
assert_eq!(text.slice(), b"hi");
```

## Error messages

Leaf parsers report what they expected and what they found. What was found
is read from the input when the error is displayed:

```rust
use parsicomb::prelude::*;
use parsicomb::utf8::is_string;

let error = is_string("hello").parse(ByteCursor::new(b"help")).unwrap_err();
println!("{error}");
```

prints

```text
Syntax error at line 1, column 4 (offset 3): expected "hello", found "help"

  > 1 | help
           ^--- here
```

Lines and columns count from 1, and the column matches the `^` pointer. The
offset is the index into the input: bytes for a byte cursor, elements
otherwise.

- **`or`** reports the alternative that got furthest into the input.
- **`and`** reports whichever side failed.
- **`label`** names a set of alternatives. When all of them fail at the start,
  the error becomes `expected <label>, found …`.

Failures past the start are kept, including real problems such as an
overflowing number or invalid UTF-8:

```rust
use parsicomb::prelude::*;
use parsicomb::ascii::u64;
use parsicomb::utf8::is_string;

let literal = is_string("true")
    .or(is_string("false"))
    .or(is_string("null"))
    .label("a literal");
let error = literal.parse(ByteCursor::new(b"maybe")).unwrap_err();
assert!(error.to_string().contains("expected a literal, found 'm'"));

let error = u64().label("a number").parse(ByteCursor::new(b"99999999999999999999")).unwrap_err();
assert!(error.to_string().contains("number too large"));
```

## Controlling backtracking

The default suits most grammars; `commit` makes failures final where the
input can only mean one thing, and `attempt` makes them "nothing here" where
a partial match should be backtracked out of.

### The default

A parser that fails **without consuming input** means "nothing here". `or`
tries its next alternative, and `optional` and the repetitions (`many`,
`some`, `separated_list`, …) stop.

A parser that fails **after consuming input** means "something here is
wrong". `optional` and the repetitions pass the error on instead of
stopping early, so the error points at the problem (the rule Parsec uses).
`or` still tries its other alternatives, and reports the one that got
furthest if they all fail:

```rust
use parsicomb::prelude::*;
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;
use parsicomb::many;

// `x=<number>;` repeated
let assignment = is_byte(b'x').and(is_byte(b'=')).and(u64()).and(is_byte(b';'));
let error = many(assignment).parse(ByteCursor::new(b"x=1;x=;")).unwrap_err();
assert!(error.to_string().contains("offset 6): expected a digit, found ';'"));
```

So `optional(separated_list(item, comma))` reads an empty or non-empty list,
and fails at a malformed item rather than at the closing bracket. Literals
are matched as a whole: `is_string("pub")` on `push` counts as "nothing
here".

### Stricter: `commit`

Once the input can only mean one thing, typically after a keyword or an
opening delimiter, `commit` makes a failure final. Inside `p.commit()`, a
failure stops `or` from trying its other alternatives, and `optional` and the
repetitions pass it on even where it matched nothing. This also saves
re-parsing alternatives that can't match, and with a commit after the first
token of each alternative, `or` behaves as in Parsec.

```rust
use parsicomb::prelude::*;
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;

// `#` and a number, or a bare `#`
let bare = || is_byte(b'#').map(|_| 0);

let tagged = is_byte(b'#').and(u64()).map(|(_, n)| n);
assert_eq!(tagged.or(bare()).parse(ByteCursor::new(b"#x")).unwrap().0, 0);

// After `#`, a number is required
let tagged = is_byte(b'#').and(u64().commit()).map(|(_, n)| n);
assert!(tagged.or(bare()).parse(ByteCursor::new(b"#x")).is_err());
```

Commit after the first token that decides, not around a whole repeated item.
In `many(item.commit())`, the item's final "nothing here" is committed too,
so the repetition fails at the end of every list.

### Looser: `attempt`

To backtrack fully from a parser instead, use `p.attempt()` (Parsec's
`try`). `optional` and the repetitions then treat any failure inside `p` as
"nothing here", while the error still points where `p` failed. A committed
failure inside stays committed.

```rust
use parsicomb::prelude::*;
use parsicomb::byte::is_byte;
use parsicomb::optional;
use parsicomb::utf8::is_string;

// `pub ` matches `pub`, then fails at `(`
let input = ByteCursor::new(b"pub(crate)");
let pub_space = || is_string("pub").and(is_byte(b' '));

assert!(optional(pub_space()).parse(input).is_err());
assert!(optional(pub_space().attempt()).parse(input).unwrap().0.is_none());
```

### Summary

What each combinator does when its parser fails:

| A failure that… | consumed nothing | consumed input | is committed |
|---|---|---|---|
| `or` | tries the next alternative | tries the next alternative | passes it on |
| `optional`, repetitions | `None` / ends | passes it on | passes it on |
| `commit` | commits it | commits it | passes it on |
| `attempt` | passes it on | makes it "nothing here" | passes it on |
| `label`, `filter` | relabels it if it failed at the start | keeps it | keeps it |
| `not` | succeeds | succeeds | passes it on |
| `and` and other sequences | passes it on | passes it on | passes it on |

A value that `filter` rejects counts as consumed nothing.

**`optional` vs `or`:** `optional(p)` is not `p.map(Some).or(default(None))`.
When `p` consumed input before failing, `or` drops that error and backtracks,
while `optional` passes it on.

**`filter` vs `invalid_error`:** `filter` rejects a value as "nothing here",
so `optional` and the repetitions stop there quietly. For a check that should
be a hard error, such as a number out of range, build the error after the
value with `cursor.invalid_error(..)` in `and_then` or `parser_fn`.

Hand-written combinators can make the same decisions with
`error.progress().since(start)`. The `error` module docs describe the model
in full.

## Untrusted input

Each nesting level of the input uses more stack, so deeply nested input such
as `[[[[…` can overflow the stack and abort the process. Bound the nesting
with `max_depth`, which fails with "maximum nesting depth exceeded" instead,
and ends the parse:

```rust
use parsicomb::prelude::*;
use parsicomb::byte::is_byte;
use parsicomb::{lazy, many};

// Nested brackets such as `[[][]]`, at most 64 levels deep
fn brackets<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = usize, Error = ParsicombError<'code>> {
    many(lazy(|| brackets().boxed()).max_depth(64))
        .between(is_byte(b'['), is_byte(b']'))
        .map(|inner| inner.len())
        .map_err(ParsicombError::from)
}

assert_eq!(brackets().parse(ByteCursor::new(b"[[][]]")).unwrap().0, 2);

let deep = "[".repeat(1000) + &"]".repeat(1000);
let error = brackets().parse(ByteCursor::new(deep.as_bytes())).unwrap_err();
assert!(error.to_string().contains("maximum nesting depth exceeded"));
```

A JSON-like grammar uses about 3 KB of stack per level in release builds and
up to 10 KB in debug builds. Spawned threads, including test threads, have
2 MiB, so keep the limit around 100 or lower there. The limit is per thread,
shared by all `max_depth` parsers on it, so use one per grammar.

`max_depth` bounds stack use, not time. When `or` backtracks, its next
alternative starts over. If two alternatives begin with the same recursive
rule, such as `'(' expr ')' '+' x` and `'(' expr ')'`, that rule is parsed
twice at every nesting level, so the time doubles with each level. Parse the
shared beginning once and make the rest optional: `'(' expr ')'` followed by
`optional('+' x)`.

## Hand-written parsers

A parser is anything implementing `Parser`. Its `parse` takes a cursor (a
`Copy` position in the input) and returns the output together with the
cursor after what it consumed:

```text
fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error>
```

On failure the caller still has its original cursor, which is what makes
backtracking free. `parser_fn` turns a closure of that shape into a parser,
for example to pick a rule by the next element without trying each in turn:

```rust
use parsicomb::prelude::*;
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;
use parsicomb::parser_fn;

fn maybe_number<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = Option<u64>, Error = ParsicombError<'code>> {
    let number = u64().map(Some);
    let none = is_byte(b'-').map(|_| None);
    parser_fn(move |cursor: ByteCursor<'code>| match cursor.value()? {
        b'-' => none.parse(cursor),
        _ => number.parse(cursor),
    })
}

assert_eq!(maybe_number().parse(ByteCursor::new(b"-")).unwrap().0, None);
assert_eq!(maybe_number().parse(ByteCursor::new(b"42")).unwrap().0, Some(42));
```

The closure's cursor argument needs its type written out
(`|cursor: ByteCursor<'code>|`), since that is where the parser's cursor
type comes from. Inside it, `?` works on any parser built from this crate's
parsers and combinators.

Cursors have more to offer here:
- `cursor.value()` reads the current element;
- `cursor.next()` and `cursor.advance(n)` move ahead;
- `cursor.position()` gives the offset;
- `cursor.expected_error(..)` and `cursor.invalid_error(..)` build errors at
  the cursor.

## Custom element types

Any plain (non-borrowing) `Copy + PartialEq + Debug + Display + Send + Sync`
type can be parsed by implementing `Atomic`, such as tokens produced by a
lexer:

```rust
use parsicomb::{AndExt, Atomic, AtomicCursor, Parser, is_atomic, is_sequence};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Let,
    Ident,
    Newline,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Atomic for Token {
    fn is_newline(&self) -> bool {
        *self == Token::Newline
    }
}

// Error messages show tokens by their `Display`, separated by spaces; override
// `format_slice`, `describe` or `unit_len` to change that
let tokens = [Token::Let, Token::Let];
let error = is_atomic(Token::Let)
    .and(is_atomic(Token::Ident))
    .parse(AtomicCursor::new(&tokens))
    .unwrap_err();

assert!(error.to_string().contains("expected Ident, found Let"));

// Runs of elements, from a static slice so failures don't allocate
static LET_IDENT: [Token; 2] = [Token::Let, Token::Ident];
let error = is_sequence(&LET_IDENT[..])
    .parse(AtomicCursor::new(&tokens))
    .unwrap_err();

assert!(error.to_string().contains(r#"expected "Let Ident", found "Let Let""#));
```

Elements are owned (`'static`), so a token can't borrow its text from the
source. Give tokens a span (start and end offsets) instead, and read the text
from the source when it's needed.

## Performance and compile times

- **Combinators inline.** A chain of combinators compiles into one function,
  and a loop such as `many(digit())` runs well under a nanosecond per byte.
- **Failed attempts cost a little.** Each one (the end of a `many`, an `or`
  alternative that doesn't match) builds an error.
- **In a hot loop, use run parsers.** `skip_while` and `skip_chars_while`
  loop internally. If profiling shows a hot spot of tiny parsers, write it
  with `parser_fn`.
- **Keep expressions short.** Combinators are nested generic types: around
  40–50 combinators in one expression, compile time grows steeply and the
  compiler's recursion limit is reached. Split long chains into functions
  returning `impl Parser`.
- **Don't make a grammar generic over closures.** Every distinct parser type
  is compiled separately, so a grammar generic over closures passed in by
  each caller is compiled once per caller. Take a trait implementation
  instead, or erase the varying part with `.boxed()`.

## License

MIT
