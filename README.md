# ParsiComb

A parser combinator library for Rust with borrowed input and rich error reporting.

## Features

- **Composable**: build complex parsers from small ones with `and`, `or`, `map`, `many`, ...
- **Borrowed input**: cursors, spans and errors borrow the input; `recognize` and
  `with_position` give the matched input as a span instead of a copy
- **Statically dispatched**: combinators are plain generic types the compiler can inline;
  type erasure with `.boxed()` is opt-in
- **Rich error messages**: line, column, offset, a context snippet and a pointer to the
  failure; messages are only formatted when an error is displayed
- **Generic over the input**: parse bytes, UTF-8 text or your own token types
- **No panics while parsing**: failures are reported through `Result`

## Quick Start

```rust
use parsicomb::{AndExt, ByteCursor, MapExt, OrExt, Parser};
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;
use parsicomb::utf8::is_string;

// Parse a number
let (value, _) = u64().parse(ByteCursor::new(b"42")).unwrap();
assert_eq!(value, 42);

// Parse alternatives
let boolean = is_string("true")
    .map(|_| true)
    .or(is_string("false").map(|_| false));
assert_eq!(boolean.parse(ByteCursor::new(b"false")).unwrap().0, false);

// Sequence parsers
let pair = u64()
    .and(is_byte(b','))
    .and(u64())
    .map(|((a, _), b)| (a, b));
assert_eq!(pair.parse(ByteCursor::new(b"10,20")).unwrap().0, (10, 20));
```

## Core Concepts

### Cursors

A cursor is a position in the input. It is `Copy`, so saving a cursor is all it
takes to backtrack. `ByteCursor` walks a byte slice; `AtomicCursor<T>` walks a
slice of any element type implementing `Atomic`.

```rust
use parsicomb::{ByteCursor, Cursor};

let cursor = ByteCursor::new(b"hello world");
let saved = cursor;
let advanced = cursor.next();

assert_eq!(saved.value().unwrap(), b'h');
assert_eq!(advanced.value().unwrap(), b'e');
```

### The `Parser` trait

```rust
# use parsicomb::{Cursor, ErrorNode};
pub trait Parser {
    type Cursor: Cursor;
    type Output;
    type Error: ErrorNode<Element = <Self::Cursor as Cursor>::Element>;

    fn parse(&self, cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error>;
}
```

A parser takes a cursor and returns its output together with the cursor after
the input it consumed. On failure the caller keeps its original cursor, which
is what makes backtracking free.

A parser succeeds as soon as it has matched, even if input is left over. To
require the whole input, check the returned cursor's `eos()`, or parse the
top level with `all(item)`, which repeats `item` up to the end of the input.

Combinator methods such as `.and()`, `.or()` and `.map()` come from extension
traits (`AndExt`, `OrExt`, `MapExt`, ...), re-exported at the crate root
together with the free functions.

Parser-building functions return `impl Parser`, naming the cursor, output and
error. Naming the error matters once a grammar is recursive, and lets
callers use `?` on the parser inside `parser_fn`. A combinator's error
converts into `ParsicombError` without allocating:

```rust
use parsicomb::{ByteCursor, MapErrExt, Parser, ParsicombError};
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;
use parsicomb::separated_list;

fn numbers<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = Vec<u64>, Error = ParsicombError<'code>> {
    separated_list(u64(), is_byte(b',')).map_err(ParsicombError::from)
}

assert_eq!(numbers().parse(ByteCursor::new(b"1,2,3")).unwrap().0, vec![1, 2, 3]);
```

The `'code` in the cursor type ties a built parser to one input: call
`numbers()` again for each input rather than keeping one parser around.
Building a parser is cheap (it allocates only for `boxed`), but parsers that
contain `boxed` or `lazy` are not `Send` or `Sync`, so each thread builds its
own.

## Basic Parsers

| Parser | Matches | Output |
|--------|---------|--------|
| `byte::byte()` | any byte | `u8` |
| `byte::is_byte(b)` | the byte `b` | `u8` |
| `byte::between_bytes(lo, hi)` | a byte in `lo..=hi` | `u8` |
| `utf8::char()` | any UTF-8 character | `char` |
| `utf8::is_char(c)` | the character `c` | `char` |
| `utf8::is_string(s)` | the string `s` | `Cow<'static, str>` |
| `utf8::unicode_letter()` and friends | a Unicode character class | `char` |
| `ascii::u64()`, `ascii::i64()` | an integer | `u64`, `i64` |
| `ascii::f64()` | a float in `int.frac` form | `f64` |
| `ascii::whitespace()` | a space, tab, `\n` or `\r` | `u8` |
| `atomic()`, `is_atomic(e)`, `atomic_between(lo, hi)` | the same, for any element type | the element |
| `is_sequence(&[...])` | a run of elements | the matched `Span` |

## Combinators

```rust
use parsicomb::{ByteCursor, Parser, between, many, optional, separated_pair, some};
use parsicomb::ascii::{digit, u64};
use parsicomb::byte::is_byte;
use parsicomb::utf8::is_string;

let cursor = |input: &'static [u8]| ByteCursor::new(input);

// Zero or more, one or more
assert_eq!(many(digit()).parse(cursor(b"")).unwrap().0, vec![]);
assert_eq!(some(digit()).parse(cursor(b"12")).unwrap().0, vec![b'1', b'2']);
assert!(some(digit()).parse(cursor(b"")).is_err());

// Optional
assert_eq!(optional(is_byte(b'-')).parse(cursor(b"5")).unwrap().0, None);

// Delimited and paired (no whitespace skipping)
let bracketed = between(is_byte(b'['), u64(), is_byte(b']'));
assert_eq!(bracketed.parse(cursor(b"[42]")).unwrap().0, 42);

let pair = separated_pair(is_string("key"), is_byte(b'='), u64());
assert_eq!(pair.parse(cursor(b"key=42")).unwrap().0, ("key".into(), 42));
```

Also available: `argument_list` (`(a, b, c)`-style lists), `filter`, `label`,
`commit` and `skip_then` (see below), `not` (negative lookahead), `take_until`,
`ntimes`, `all`, `map_err` and `with_position` (captures the `Span` a parser
consumed). A parser can be used in several places by reference:
`item.and(&separator)`.

`a.or(b)` tries `b` if `a` fails, and takes the first that succeeds.

`optional` and the repetitions (`many`, `some`, `skip_many`, `separated_list`,
...) stop where their parser matches nothing (see
[How failures are handled](#how-failures-are-handled)). Once it has matched part of its
input, a failure means the input is wrong, so they pass it on instead of
stopping early, and the error points at the problem (the rule Parsec uses).
Real problems, such as a number too large, are passed on too:

```rust
use parsicomb::{AndExt, ByteCursor, Parser, many};
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;

// `x=<number>;` repeated
let assignment = is_byte(b'x').and(is_byte(b'=')).and(u64()).and(is_byte(b';'));
let error = many(assignment).parse(ByteCursor::new(b"x=1;x=;")).unwrap_err();
assert!(error.to_string().contains("offset 6): expected a digit, found ';'"));
```

So `optional(separated_list(item, comma))` reads an empty or non-empty list,
and fails at a malformed item rather than at the closing bracket. Two
patterns follow from the rule:

- Skip whitespace after each token (`token.and(whitespace)`), so a token that
  isn't there matches nothing. In grammars that skip whitespace before tokens,
  use `skip_then(whitespace, token)`, which doesn't count the skipped
  whitespace as a match.
- Match keywords as whole words, e.g. `identifier().filter(|w| w == "let",
  "`let`")`, rather than `is_string("let")` followed by more: on `letter`, the
  latter has matched `let` and fails, where the filter matches nothing.

Repetitions also stop when an iteration succeeds without consuming input,
since it would repeat forever; `all` and `ntimes` fail in that case instead.

Backtracking re-parses: when `a` fails, `b` starts over from the same place.
If both alternatives begin with the same recursive rule, such as
`'(' expr ')' '+' x` or `'(' expr ')'`, that rule is parsed twice at every
nesting level, and the time doubles with each level (20 levels already take
about 300 ms). Parse the shared beginning once and make the difference
optional: `'(' expr ')'` followed by `optional('+' x)`.

### Runs without collecting

`many` and `some` collect their outputs into a `Vec`. When only the matched
input matters, `skip_many`/`skip_some` repeat without collecting, and
`recognize` outputs the span a parser matched:

```rust
use parsicomb::{ByteCursor, FilterExt, Parser, recognize, skip_some};
use parsicomb::byte::byte;

let ident = recognize(skip_some(
    byte().filter(|b| b.is_ascii_alphanumeric() || *b == b'_', "an identifier character"),
));
let (span, _) = ident.parse(ByteCursor::new(b"foo_bar baz")).unwrap();
assert_eq!(span.slice(), b"foo_bar");
```

For runs of single elements or characters, `skip_while` (any element type) and
`utf8::skip_chars_while` check each one with a predicate. Unlike
`skip_many(..filter(..))` they build no error where the run ends, which
matters in hot loops such as lexers:

```rust
use parsicomb::{ByteCursor, Parser, recognize};
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
use parsicomb::{AndExt, AndThenExt, ByteCursor, FilterExt, MapExt, Parser};
use parsicomb::{many, ntimes, recognize, skip_many};
use parsicomb::byte::{byte, is_byte};

let raw = many(is_byte(b'#'))
    .map(|hashes| hashes.len())
    .and_then(|n| {
        is_byte(b'"')
            .and(recognize(skip_many(byte().filter(|b| *b != b'"', "text"))))
            .and(is_byte(b'"'))
            .and(ntimes(is_byte(b'#'), n))
            .map(|(((_, text), _), _)| text)
    });

let (text, _) = raw.parse(ByteCursor::new(b"##\"hi\"##")).unwrap();
assert_eq!(text.slice(), b"hi");
```

### Hand-written parsers

`parser_fn` turns a closure from a cursor to a parse result into a parser,
e.g. to pick a rule by the next element without trying each in turn:

```rust
use parsicomb::{ByteCursor, Cursor, MapExt, Parser, ParsicombError, parser_fn};
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;

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
```

Inside the closure, `?` works on any parser built from this crate's parsers
and combinators: their errors convert into `ParsicombError`. The closure's
cursor argument needs its type written out
(`|cursor: ByteCursor<'code>|`), since that is where the parser's cursor type
comes from. Cursors can also skip ahead: `cursor.advance(n)` moves `n`
elements in one step.

## Recursive Grammars

A parser that refers to itself goes through `lazy`, which builds the inner
parser on first use and caches it. At that point the parser type must be
erased with `.boxed()`, and the error type must be concrete, since both would
otherwise contain themselves. `map_err(ParsicombError::from)` makes it
concrete without allocating: the error of a combinator converts into the
`ParsicombError` of whichever part failed. Errors of other types, such as a
`MapErr` to your own error, can be boxed with `ParsicombError::wrap` instead,
which allocates once per failure:

```rust
use parsicomb::{AndExt, BoxedExt, ByteCursor, MapErrExt, MapExt, OrExt, Parser};
use parsicomb::{ParsicombError, lazy, separated_list};
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;

#[derive(Debug, PartialEq)]
enum Value {
    Number(u64),
    List(Vec<Value>),
}

fn value<'code>()
-> impl Parser<Cursor = ByteCursor<'code>, Output = Value, Error = ParsicombError<'code>> {
    let number = u64().map(Value::Number);
    let list = is_byte(b'[')
        .and(separated_list(lazy(|| value().boxed()), is_byte(b',')))
        .and(is_byte(b']'))
        .map(|((_, items), _)| Value::List(items));

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

Each nesting level of the input uses more stack, so deeply nested input such as
`[[[[…` can overflow the stack and abort the process. When parsing untrusted
input, bound the nesting with `max_depth`, which fails with "maximum nesting
depth exceeded" instead. That error is committed (see `commit` below), so it
ends the parse. The count is per thread and shared by all `max_depth` parsers
on it, so use one limit per grammar.

How much stack a level takes depends on the grammar and the build: a
JSON-like grammar uses about 3 KB per level in release builds and up to 10 KB
in debug builds. Spawned threads, including test threads, have 2 MiB of stack,
so keep the limit around 100 or lower there, or run the parser on a thread
with a larger stack:

```rust
# use parsicomb::{BoxedExt, ByteCursor, Parser, lazy};
use parsicomb::MaxDepthExt;
# fn value<'code>() -> impl Parser<Cursor = ByteCursor<'code>, Output = u8, Error = parsicomb::ParsicombError<'code>> { parsicomb::byte::byte() }
let element = lazy(|| value().boxed()).max_depth(64);
```

`max_depth` bounds stack use, not time: a grammar whose alternatives share a
recursive beginning can still take exponential time well within the limit
(see [Combinators](#combinators)).

## Custom Element Types

Any plain (non-borrowing) `Copy + PartialEq + Debug + Display + Send + Sync`
type can be parsed by implementing `Atomic`, e.g. tokens produced by a lexer:

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

## Error Messages

Leaf parsers report what they expected and what they found, read from the
input when the error is displayed:

```rust
use parsicomb::{ByteCursor, Parser};
use parsicomb::utf8::is_string;

let error = is_string("hello").parse(ByteCursor::new(b"help")).unwrap_err();

assert_eq!(
    error.to_string(),
    "Syntax error at line 1, column 4 (offset 3): \
     expected \"hello\", found \"help\"\n\
     \n  > 1 | help\n           ^--- here\n"
);
```

Lines and columns count from 1. `column` counts the characters of the
rendered line up to the error, so it matches the `^` pointer (the snippet
shows at most 512 elements on each side, so a longer line is cut); `offset` is the
absolute index into the input (bytes for a byte cursor, elements otherwise).
For errors more than 2^20 elements into the input only the offset is shown, so
rendering stays cheap.

When alternatives fail, `or` reports the one that got furthest into the input
(see "How failures are handled" below); `and` reports whichever side failed.
Name a set of alternatives with `label`: when all of them fail at the start,
the error becomes `expected <label>, found …`. Failures past the start are
kept, including real problems such as an overflowing number or invalid UTF-8,
which are reported after the input they are in:

```rust
use parsicomb::{ByteCursor, LabelExt, OrExt, Parser};
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

`or` keeps trying alternatives after one that matched part of its input.
Once the input can only mean one thing, typically after a keyword or an opening delimiter, `commit` makes a failure
final: inside `p.commit()`, a failure stops `or` from trying the remaining
alternatives, and `optional` and repetitions pass it on even where it
matched nothing. This also saves re-parsing alternatives that can't match.
Commit after the first token that decides, not around a whole repeated item:
in `many(item.commit())`, the item's final "nothing here" is committed too,
so the repetition can never end.

### How failures are handled

Every error says how far the failed parser got (`error.progress()`). Each
failure is somewhere (`at`), and the message points there. The parsers
around a failed parser also need to know how far its progress counts, to
decide whether to backtrack:

- `Progress::Partial { at }`: progress up to the failure. A leaf parser
  builds its error at the cursor where it stopped, with
  `cursor.expected_error(..)` where what it expected isn't there, or
  `cursor.invalid_error(..)` after input that is there but wrong (a number
  too large, invalid UTF-8).
- `Progress::Peek { from, at }`: it only peeked at the input from `from`
  before failing at `at`, so its progress counts only up to `from`. This is
  for input that isn't evidence the parser is on the right path: `skip_then`
  doesn't count the whitespace it skipped, so a missing token after it
  fails at the token but counts as no progress. The parsers around it see
  "nothing here" while the message still points at the missing token. Put
  `label` inside `skip_then` rather than around it, so the label replaces
  the error where the token is missing.
- `Progress::Committed { at }`: it failed inside a `commit`, or fatally
  (nesting too deep).

A combinator that started a parser at `start` sees how far it advanced
(`error.progress().since(start)`):

- `Advance::None`: no progress past the start. Nothing here.
- `Advance::Partial`: progress past the start, then a failure. Something
  here is wrong.
- `Advance::Committed`: committed or fatal.

Progress is ordered: committed first, then the progress that counts
further, then the failure further in. `or` reports the alternative with
the greater progress, so an alternative that matched nothing (and isn't
committed) is never reported over one that matched input, however far in its
error points.

| | `None` | `Partial` | `Committed` |
|---|---|---|---|
| `or` | tries the next alternative | tries the next alternative | passes it on |
| `optional`, repetitions | `None` / ends | passes it on | passes it on |
| `label`, `filter` | replaces the message if it failed at the start | keeps it | keeps it |
| `not` | succeeds | succeeds | passes it on |
| `skip_then` | a peek, if it skipped input | passes it on | passes it on |
| `and` and other sequences | passes it on | passes it on | passes it on |

So `optional(p)` is not `p.map(Some).or(default(None))`: `or` also
backtracks from `Advance::Partial` and drops that error, where `optional`
passes it on.
Hand-written combinators can make the same decisions with
`match error.progress().since(start) { .. }`.

`filter` rejects a value as "nothing here" at its start, so `optional` and
repetitions stop there quietly. For a value check that should be a hard
error, such as a number out of range, build the error after the value with
`cursor.invalid_error(..)` in `and_then` or `parser_fn`.

Leaf parsers that run out of input report `expected …, found end of input`;
reading past the end with `cursor.value()` in a hand-written parser gives
`Unexpected end of input`.

## Compile Times

Combinators are nested generic types, so a single expression with dozens of
combinators makes for deep types: compile time grows steeply, and around 40–50
combinators the compiler's recursion limit is reached. Split long chains into
functions returning `impl Parser`, and erase types with `.boxed()` at a few
boundaries if needed.

Every distinct parser type is compiled separately. Keep a grammar's type
parameters to things that are the same across call sites: a grammar that is
generic over closures passed in by each caller is compiled once per caller,
since every closure has its own type. Take a trait implementation (one type
shared by all callers) instead, or erase the varying part with `.boxed()`.
