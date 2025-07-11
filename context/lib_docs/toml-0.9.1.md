Title: toml - Rust

URL Source: https://docs.rs/toml/latest/toml/

Markdown Content:
Expand description

A [serde](https://serde.rs/)-compatible [TOML](https://github.com/toml-lang/toml)-parsing library

TOML itself is a simple, ergonomic, and readable configuration format:

```
[package]
name = "toml"

[dependencies]
serde = "1.0"
```

The TOML format tends to be relatively common throughout the Rust community for configuration, notably being used by [Cargo](https://crates.io/), Rust’s package manager.

### [§](https://docs.rs/toml/latest/toml/#toml-values)TOML values

A TOML document is represented with the [`Table`](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table") type which maps `String` to the [`Value`](https://docs.rs/toml/latest/toml/enum.Value.html "enum toml::Value") enum:

```
pub enum Value {
   String(String),
   Integer(i64),
   Float(f64),
   Boolean(bool),
   Datetime(Datetime),
   Array(Array),
   Table(Table),
}
```

### [§](https://docs.rs/toml/latest/toml/#parsing-toml)Parsing TOML

The easiest way to parse a TOML document is via the [`Table`](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table") type:

```
use toml::Table;

let value = "foo = 'bar'".parse::<Table>().unwrap();

assert_eq!(value["foo"].as_str(), Some("bar"));
```

The [`Table`](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table") type implements a number of convenience methods and traits; the example above uses [`FromStr`](https://doc.rust-lang.org/nightly/core/str/traits/trait.FromStr.html "trait core::str::traits::FromStr") to parse a [`str`](https://doc.rust-lang.org/nightly/std/primitive.str.html "primitive str") into a [`Table`](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table").

### [§](https://docs.rs/toml/latest/toml/#deserialization-and-serialization)Deserialization and Serialization

This crate supports [`serde`](https://serde.rs/) 1.0 with a number of implementations of the `Deserialize`, `Serialize`, `Deserializer`, and `Serializer` traits. Namely, you’ll find:

- `Deserialize for Table`
- `Serialize for Table`
- `Deserialize for Value`
- `Serialize for Value`
- `Deserialize for Datetime`
- `Serialize for Datetime`
- `Deserializer for de::Deserializer`
- `Serializer for ser::Serializer`
- `Deserializer for Table`
- `Deserializer for Value`

This means that you can use Serde to deserialize/serialize the [`Table`](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table") type as well as [`Value`](https://docs.rs/toml/latest/toml/enum.Value.html "enum toml::Value") and [`Datetime`](https://docs.rs/toml/latest/toml/value/struct.Datetime.html "struct toml::value::Datetime") type in this crate. You can also use the [`Deserializer`](https://docs.rs/toml/latest/toml/struct.Deserializer.html "struct toml::Deserializer"), [`Serializer`](https://docs.rs/toml/latest/toml/struct.Serializer.html "struct toml::Serializer"), or [`Table`](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table") type itself to act as a deserializer/serializer for arbitrary types.

An example of deserializing with TOML is:

```
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
   ip: String,
   port: Option<u16>,
   keys: Keys,
}

#[derive(Deserialize)]
struct Keys {
   github: String,
   travis: Option<String>,
}

let config: Config = toml::from_str(r#"
   ip = '127.0.0.1'

   [keys]
   github = 'xxxxxxxxxxxxxxxxx'
   travis = 'yyyyyyyyyyyyyyyyy'
"#).unwrap();

assert_eq!(config.ip, "127.0.0.1");
assert_eq!(config.port, None);
assert_eq!(config.keys.github, "xxxxxxxxxxxxxxxxx");
assert_eq!(config.keys.travis.as_ref().unwrap(), "yyyyyyyyyyyyyyyyy");
```

You can serialize types in a similar fashion:

```
use serde::Serialize;

#[derive(Serialize)]
struct Config {
   ip: String,
   port: Option<u16>,
   keys: Keys,
}

#[derive(Serialize)]
struct Keys {
   github: String,
   travis: Option<String>,
}

let config = Config {
   ip: "127.0.0.1".to_string(),
   port: None,
   keys: Keys {
       github: "xxxxxxxxxxxxxxxxx".to_string(),
       travis: Some("yyyyyyyyyyyyyyyyy".to_string()),
   },
};

let toml = toml::to_string(&config).unwrap();
```

[de](https://docs.rs/toml/latest/toml/de/index.html "mod toml::de")Deserializing TOML into Rust structures.[map](https://docs.rs/toml/latest/toml/map/index.html "mod toml::map")A map of `String` to [Value](https://docs.rs/toml/latest/toml/enum.Value.html "enum toml::Value").[ser](https://docs.rs/toml/latest/toml/ser/index.html "mod toml::ser")`serde`Serializing Rust structures into TOML.[value](https://docs.rs/toml/latest/toml/value/index.html "mod toml::value")`serde`Definition of a TOML [value](https://docs.rs/toml/latest/toml/enum.Value.html "enum toml::Value")[toml](https://docs.rs/toml/latest/toml/macro.toml.html "macro toml::toml")`serde`Construct a [`Table`](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table") from TOML syntax.[Deserializer](https://docs.rs/toml/latest/toml/struct.Deserializer.html "struct toml::Deserializer")`parse` and `serde`Deserialization for TOML [documents](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table").[Serializer](https://docs.rs/toml/latest/toml/struct.Serializer.html "struct toml::Serializer")`display` and `serde`Serialization for TOML documents.[Spanned](https://docs.rs/toml/latest/toml/struct.Spanned.html "struct toml::Spanned")A spanned value, indicating the range at which it is defined in the source.[Value](https://docs.rs/toml/latest/toml/enum.Value.html "enum toml::Value")`serde`Representation of a TOML value.[from\_ slice](https://docs.rs/toml/latest/toml/fn.from_slice.html "fn toml::from_slice")`parse` and `serde`Deserializes bytes into a type.[from\_ str](https://docs.rs/toml/latest/toml/fn.from_str.html "fn toml::from_str")`parse` and `serde`Deserializes a string into a type.[to\_ string](https://docs.rs/toml/latest/toml/fn.to_string.html "fn toml::to_string")`serde` and `display`Serialize the given data structure as a String of TOML.[to* string* pretty](https://docs.rs/toml/latest/toml/fn.to_string_pretty.html "fn toml::to_string_pretty")`serde` and `display`Serialize the given data structure as a “pretty” String of TOML.[Table](https://docs.rs/toml/latest/toml/type.Table.html "type toml::Table")`serde`Type representing a TOML table, payload of the `Value::Table` variant.
