# jawson

Jawson is a JSON parser built from scratch in Rust as a learning project to
explore parsing techniques.

> Note: this is a learning project, not a production parser. For real work use
> [`serde_json`](https://crates.io/crates/serde_json). See [Limitations](#limitations).

## Features

- Parses every JSON value type: `null`, booleans, numbers, strings, arrays, objects.
- Numbers are classified as integers (`i64`) or floats (`f64`) based on the source.
- Full string escape handling: `\" \\ \/ \b \f \n \r \t`, `\uXXXX`, and surrogate
  pairs (for example `😀` decodes to an emoji).
- Recursive descent parsing driven by single byte lookahead.
- Typed error reporting through `ParseError`.
- No unsafe code; only the standard library plus `thiserror`.

## Usage

```rust
use jawson::{parse, JsonValue};

let json = r#"
    {
        "name": "Ada Lovelace",
        "age": 36,
        "active": true,
        "languages": ["rust", "ml", null],
        "address": { "city": "London", "zip": "SW1" }
    }
"#;

let value = parse(json).unwrap();
println!("{value:?}");

if let JsonValue::Object(map) = &value {
    assert_eq!(map["age"], JsonValue::Int(36));
    assert_eq!(map["active"], JsonValue::Bool(true));
    assert_eq!(
        map["languages"],
        JsonValue::Array(vec![
            JsonValue::String("rust".to_string()),
            JsonValue::String("ml".to_string()),
            JsonValue::Null,
        ]),
    );
}
```

## The value type

```rust
pub enum JsonValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}
```

## Errors

Invalid input returns a `ParseError`, with variants including `UnexpectedByte`,
`UnexpectedEof`, `InvalidEscape`, `InvalidUnicode`, `InvalidInt`, `InvalidFloat`,
and `InvalidUtf8`.

```rust
use jawson::{parse, ParseError};

// a truncated object: input ends before the closing brace
assert_eq!(parse(r#"{"a": 1"#), Err(ParseError::UnexpectedEof));
```

## Building and testing

```sh
cargo build
cargo test          # unit tests and doc-tests
cargo doc --open    # rendered API docs
```

## Limitations

This parser is intentionally a learning exercise. Known gaps versus a production
parser:

- The grammar is not strictly enforced: missing commas (`[1 2]`), extra commas
  (`[1,,2]`), and trailing data after the top level value are currently accepted.
- There is no recursion depth limit, so deeply nested input can overflow the stack.
- Errors do not carry a line or column position.
- Input is read through `BufReader` one byte at a time with a fresh allocation per
  value, so it is much slower than an indexing parser.
- It has not been validated against the standard JSON test suite.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
