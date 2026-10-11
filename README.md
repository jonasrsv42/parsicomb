# ParsiComb

A parser combinator library for Rust with zero-copy parsing and rich error reporting.

## Features

- **Composable**: build complex parsers from small ones with `and`, `or`, `map`, `many`, ...
- **Zero-copy**: cursors, spans and errors borrow the input; nothing is copied out of it
- **Statically dispatched**: combinators are plain generic types the compiler can inline;
  type erasure with `.boxed()` is opt-in
- **Rich error messages**: line, column, offset, a context snippet and a pointer to the
  failure; messages are only built when an error is displayed, so failed alternatives are cheap
- **Generic over the input**: parse bytes, UTF-8 text or your own token types
- **No panics while parsing**: failures are reported through `Result`

## Quick Start

```rust
use parsicomb::{ByteCursor, Parser};
use parsicomb::and::AndExt;
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;
use parsicomb::map::MapExt;
use parsicomb::or::OrExt;
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

Parser-building functions return `impl Parser`, naming the cursor and output:

```rust
use parsicomb::{ByteCursor, Parser};
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;
use parsicomb::separated_list;

fn numbers<'code>() -> impl Parser<Cursor = ByteCursor<'code>, Output = Vec<u64>> {
    separated_list(u64(), is_byte(b','))
}

assert_eq!(numbers().parse(ByteCursor::new(b"1,2,3")).unwrap().0, vec![1, 2, 3]);
```

## Basic Parsers

| Parser | Matches | Output |
|--------|---------|--------|
| `byte::byte()` | any byte | `u8` |
| `byte::is_byte(b)` | the byte `b` | `u8` |
| `byte::between_bytes(lo, hi)` | a byte in `lo..=hi` | `u8` |
| `utf8::char()` | any UTF-8 character | `char` |
| `utf8::char::is_char(c)` | the character `c` | `char` |
| `utf8::is_string(s)` | the string `s` | `Cow<'static, str>` |
| `utf8::unicode_letter()` and friends | a Unicode character class | `char` |
| `ascii::u64()`, `ascii::i64()` | an integer | `u64`, `i64` |
| `ascii::f64()` | a float in `int.frac` form | `f64` |
| `ascii::whitespace()` | a space, tab, `\n` or `\r` | `u8` |
| `atomic()`, `is_atomic(e)`, `atomic_between(lo, hi)` | the same, for any element type | the element |

## Combinators

```rust
use parsicomb::{ByteCursor, Parser, between, optional, separated_pair};
use parsicomb::ascii::{digit, u64};
use parsicomb::byte::is_byte;
use parsicomb::many::many;
use parsicomb::some::some;
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

Also available: `argument_list` (`(a, b, c)`-style lists), `filter`, `not`
(negative lookahead), `take_until`, `ntimes`, `all`, `map_err` and
`with_position` (captures the `Span` a parser consumed).

## Recursive Grammars

A parser that refers to itself goes through `lazy`, which builds the inner
parser on first use and caches it. At that point the parser type must be
erased with `.boxed()`, and the error type must be concrete, since both would
otherwise contain themselves:

```rust
use parsicomb::{BoxedExt, ByteCursor, Parser, ParsicombError, lazy, separated_list};
use parsicomb::and::AndExt;
use parsicomb::ascii::u64;
use parsicomb::byte::is_byte;
use parsicomb::map::MapExt;
use parsicomb::map_err::MapErrExt;
use parsicomb::or::OrExt;

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

    number.or(list).map_err(ParsicombError::wrap)
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

## Custom Element Types

Any `Copy + PartialEq + Debug + Display` type can be parsed by implementing
`Atomic`, e.g. tokens produced by a lexer:

```rust
use parsicomb::{Atomic, AtomicCursor, Parser, is_atomic};
use parsicomb::and::AndExt;
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

    fn format_slice(slice: &[Self]) -> String {
        slice.iter().map(|t| format!("{} ", t)).collect()
    }
}

let tokens = [Token::Let, Token::Let];
let error = is_atomic(Token::Let)
    .and(is_atomic(Token::Ident))
    .parse(AtomicCursor::new(&tokens))
    .unwrap_err();

assert!(error.to_string().contains("expected Ident, found Let"));
```

## Error Messages

Errors report the furthest point any alternative reached:

```rust
use parsicomb::{ByteCursor, Parser};
use parsicomb::utf8::is_string;

let error = is_string("hello").parse(ByteCursor::new(b"help")).unwrap_err();

assert_eq!(
    error.to_string(),
    "Syntax error at line 1, column 3 (offset 3): \
     expected 'l', found 'p' while matching \"hello\"\n\
     \n  > 1 | help\n           ^--- here\n"
);
```

`column` counts display columns in the rendered line; `offset` is the absolute
index into the input (bytes for a byte cursor, elements otherwise).

## Compile Times

Combinators are nested generic types, so a single expression with dozens of
combinators makes for deep types: compile time grows steeply, and around 40–50
combinators the compiler's recursion limit is reached. Split long chains into
functions returning `impl Parser`, and erase types with `.boxed()` at a few
boundaries if needed.
