# ParsiComb

A high-performance, zero-panic parser combinator library for Rust with rich error reporting.

## Features

- **Zero Panics**: All errors handled through `Result` types
- **Rich Error Messages**: Line numbers, context, and visual error pointers
- **Performance Focused**: Byte-level operations with minimal allocations
- **UTF-8 Support**: Full Unicode parsing capabilities
- **Composable**: Build complex parsers from simple building blocks
- **Type Safe**: Parser output types are statically known

## Quick Start

```rust
use parsicomb::{byte, is_byte, is_string, u64, Parser};
use parsicomb::combinators::{and::AndExt, or::OrExt, map::MapExt};

// Parse a number
let number = u64();
let result = number.parse("42".as_bytes());

// Parse alternatives
let boolean = is_string("true").map(|_| true)
    .or(is_string("false").map(|_| false));

// Sequence parsers
let pair = u64().and(is_byte(b',')).and(u64())
    .map(|((a, _), b)| (a, b));
// Parses "10,20" → (10, 20)
```

## Core Concepts

### ByteCursor

The foundation of parsing - an immutable cursor that tracks position in a byte array:

```rust
let cursor = ByteCursor::new(b"hello world");
// Cursors can be copied for backtracking
let saved = cursor;
```

### Parser Trait

All parsers implement this trait:

```rust
pub trait Parser<'a>: Sized {
    type Output;
    fn parse(&self, cursor: ByteCursor<'a>) -> Result<(Self::Output, ByteCursor<'a>), Error>;
}
```

## Basic Parsers

| Parser | Description | Example |
|--------|-------------|---------|
| `byte()` | Consumes any byte | `byte().parse(b"a")` → `Ok((b'a', ...))` |
| `is_byte(b)` | Matches specific byte | `is_byte(b'x').parse(b"x")` → `Ok((b'x', ...))` |
| `is_string(s)` | Matches string | `is_string("hello").parse(b"hello")` → `Ok(("hello", ...))` |
| `u64()` | Parses unsigned integer | `u64().parse(b"123")` → `Ok((123, ...))` |
| `i64()` | Parses signed integer | `i64().parse(b"-42")` → `Ok((-42, ...))` |
| `f64()` | Parses floating point | `f64().parse(b"3.14")` → `Ok((3.14, ...))` |
| `char()` | Parses UTF-8 character | `char().parse("🦀".as_bytes())` → `Ok(('🦀', ...))` |

## Combinators

### Sequencing with `and()`

```rust
let parser = is_string("hello")
    .and(is_byte(b' '))
    .and(is_string("world"))
    .map(|((greeting, _), target)| format!("{} {}", greeting, target));

// Parses "hello world" → "hello world"
```

### Alternation with `or()`

```rust
let parser = is_string("yes").map(|_| true)
    .or(is_string("no").map(|_| false));

// Parses "yes" → true, "no" → false
```

### Repetition

```rust
use parsicomb::combinators::{many, some};

// Zero or more
let parser = many(digit());  // Parses "" → vec![], "123" → vec!['1', '2', '3']

// One or more
let parser = some(digit());  // Parses "123" → vec!['1', '2', '3'], "" → Error
```

### Lists and Pairs

```rust
use parsicomb::combinators::{separated_list, separated_pair, between};

// Parse comma-separated numbers
let list = separated_list(u64(), is_byte(b','));
// Parses "1,2,3" → vec![1, 2, 3]

// Parse key-value pair
let pair = separated_pair(is_string("key"), is_byte(b'='), u64());
// Parses "key=42" → ("key", 42)

// Parse with delimiters
let bracketed = between(is_byte(b'['), u64(), is_byte(b']'));
// Parses "[42]" or "[ 42 ]" → 42
```

## Error Handling

ParsiComb provides detailed error messages with context:

```rust
let parser = is_string("hello");
let result = parser.parse("world".as_bytes());

// Error output:
// Syntax error at line 1, byte offset 0: expected "hello", found "world"
//
//   > 1 | world
//         ^--- here
```

## Complete Example

```rust
use parsicomb::{ByteCursor, Parser, is_string, u64, is_byte};
use parsicomb::combinators::{
    and::AndExt, or::OrExt, map::MapExt, 
    many, separated_list, between
};

#[derive(Debug, PartialEq)]
enum Value {
    Number(u64),
    String(String),
    Array(Vec<Value>),
}

fn value_parser() -> impl Parser<'static, Output = Value> {
    let number = u64().map(Value::Number);
    
    let string = between(
        is_byte(b'"'),
        take_until(is_byte(b'"')),
        is_byte(b'"')
    ).map(|s| Value::String(String::from_utf8_lossy(s).to_string()));
    
    let array = between(
        is_byte(b'['),
        separated_list(value_parser(), is_byte(b',')),
        is_byte(b']')
    ).map(Value::Array);
    
    number.or(string).or(array)
}

// Usage
let input = r#"[42, "hello", [1, 2, 3]]"#;
let cursor = ByteCursor::new(input.as_bytes());
let result = value_parser().parse(cursor);
```

## Performance Tips

1. **Use byte-level parsers when possible**: `is_byte()` is faster than `is_string()` for single characters
2. **Avoid unnecessary allocations**: Use `map()` to transform data in-place
3. **Order alternatives by likelihood**: Put most common cases first in `or()` chains
4. **Use specialized parsers**: `u64()` is faster than manually parsing digits

