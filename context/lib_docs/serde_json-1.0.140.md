Title: serde_json - Rust

URL Source: https://docs.rs/serde_json/latest/serde_json/

Markdown Content:
Expand description

## [§](https://docs.rs/serde_json/latest/serde_json/#serde-json)Serde JSON

JSON is a ubiquitous open-standard format that uses human-readable text to transmit data objects consisting of key-value pairs.

```
{
    "name": "John Doe",
    "age": 43,
    "address": {
        "street": "10 Downing Street",
        "city": "London"
    },
    "phones": [
        "+44 1234567",
        "+44 2345678"
    ]
}
```

There are three common ways that you might find yourself needing to work with JSON data in Rust.

- **As text data.** An unprocessed string of JSON data that you receive on an HTTP endpoint, read from a file, or prepare to send to a remote server.
- **As an untyped or loosely typed representation.** Maybe you want to check that some JSON data is valid before passing it on, but without knowing the structure of what it contains. Or you want to do very basic manipulations like insert a key in a particular spot.
- **As a strongly typed Rust data structure.** When you expect all or most of your data to conform to a particular structure and want to get real work done without JSON’s loosey-goosey nature tripping you up.

Serde JSON provides efficient, flexible, safe ways of converting data between each of these representations.

## [§](https://docs.rs/serde_json/latest/serde_json/#operating-on-untyped-json-values)Operating on untyped JSON values

Any valid JSON data can be manipulated in the following recursive enum representation. This data structure is [`serde_json::Value`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html "enum serde_json::Value").

```
enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
}
```

A string of JSON data can be parsed into a `serde_json::Value` by the [`serde_json::from_str`](https://docs.rs/serde_json/latest/serde_json/fn.from_str.html "fn serde_json::from_str") function. There is also [`from_slice`](https://docs.rs/serde_json/latest/serde_json/fn.from_slice.html "fn serde_json::from_slice") for parsing from a byte slice `&[u8]` and [`from_reader`](https://docs.rs/serde_json/latest/serde_json/fn.from_reader.html "fn serde_json::from_reader") for parsing from any `io::Read` like a File or a TCP stream.

```
use serde_json::{Result, Value};

fn untyped_example() -> Result<()> {
    // Some JSON input data as a &str. Maybe this comes from the user.
    let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;

    // Parse the string of data into serde_json::Value.
    let v: Value = serde_json::from_str(data)?;

    // Access parts of the data by indexing with square brackets.
    println!("Please call {} at the number {}", v["name"], v["phones"][0]);

    Ok(())
}
```

The result of square bracket indexing like `v["name"]` is a borrow of the data at that index, so the type is `&Value`. A JSON map can be indexed with string keys, while a JSON array can be indexed with integer keys. If the type of the data is not right for the type with which it is being indexed, or if a map does not contain the key being indexed, or if the index into a vector is out of bounds, the returned element is `Value::Null`.

When a `Value` is printed, it is printed as a JSON string. So in the code above, the output looks like `Please call "John Doe" at the number "+44 1234567"`. The quotation marks appear because `v["name"]` is a `&Value` containing a JSON string and its JSON representation is `"John Doe"`. Printing as a plain string without quotation marks involves converting from a JSON string to a Rust string with [`as_str()`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html#method.as_str "method serde_json::Value::as_str") or avoiding the use of `Value` as described in the following section.

The `Value` representation is sufficient for very basic tasks but can be tedious to work with for anything more significant. Error handling is verbose to implement correctly, for example imagine trying to detect the presence of unrecognized fields in the input data. The compiler is powerless to help you when you make a mistake, for example imagine typoing `v["name"]` as `v["nmae"]` in one of the dozens of places it is used in your code.

## [§](https://docs.rs/serde_json/latest/serde_json/#parsing-json-as-strongly-typed-data-structures)Parsing JSON as strongly typed data structures

Serde provides a powerful way of mapping JSON data into Rust data structures largely automatically.

```
use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,
    phones: Vec<String>,
}

fn typed_example() -> Result<()> {
    // Some JSON input data as a &str. Maybe this comes from the user.
    let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;

    // Parse the string of data into a Person object. This is exactly the
    // same function as the one that produced serde_json::Value above, but
    // now we are asking it for a Person as output.
    let p: Person = serde_json::from_str(data)?;

    // Do things just like with any other Rust data structure.
    println!("Please call {} at the number {}", p.name, p.phones[0]);

    Ok(())
}
```

This is the same `serde_json::from_str` function as before, but this time we assign the return value to a variable of type `Person` so Serde will automatically interpret the input data as a `Person` and produce informative error messages if the layout does not conform to what a `Person` is expected to look like.

Any type that implements Serde’s `Deserialize` trait can be deserialized this way. This includes built-in Rust standard library types like `Vec<T>` and `HashMap<K, V>`, as well as any structs or enums annotated with `#[derive(Deserialize)]`.

Once we have `p` of type `Person`, our IDE and the Rust compiler can help us use it correctly like they do for any other Rust code. The IDE can autocomplete field names to prevent typos, which was impossible in the `serde_json::Value` representation. And the Rust compiler can check that when we write `p.phones[0]`, then `p.phones` is guaranteed to be a `Vec<String>` so indexing into it makes sense and produces a `String`.

## [§](https://docs.rs/serde_json/latest/serde_json/#constructing-json-values)Constructing JSON values

Serde JSON provides a [`json!` macro](https://docs.rs/serde_json/latest/serde_json/macro.json.html "macro serde_json::json") to build `serde_json::Value` objects with very natural JSON syntax.

```
use serde_json::json;

fn main() {
    // The type of `john` is `serde_json::Value`
    let john = json!({
        "name": "John Doe",
        "age": 43,
        "phones": [
            "+44 1234567",
            "+44 2345678"
        ]
    });

    println!("first phone number: {}", john["phones"][0]);

    // Convert to a string of JSON and print it out
    println!("{}", john.to_string());
}
```

The `Value::to_string()` function converts a `serde_json::Value` into a `String` of JSON text.

One neat thing about the `json!` macro is that variables and expressions can be interpolated directly into the JSON value as you are building it. Serde will check at compile time that the value you are interpolating is able to be represented as JSON.

```
let full_name = "John Doe";
let age_last_year = 42;

// The type of `john` is `serde_json::Value`
let john = json!({
    "name": full_name,
    "age": age_last_year + 1,
    "phones": [
        format!("+44 {}", random_phone())
    ]
});
```

This is amazingly convenient, but we have the problem we had before with `Value`: the IDE and Rust compiler cannot help us if we get it wrong. Serde JSON provides a better way of serializing strongly-typed data structures into JSON text.

## [§](https://docs.rs/serde_json/latest/serde_json/#creating-json-by-serializing-data-structures)Creating JSON by serializing data structures

A data structure can be converted to a JSON string by [`serde_json::to_string`](https://docs.rs/serde_json/latest/serde_json/fn.to_string.html "fn serde_json::to_string"). There is also [`serde_json::to_vec`](https://docs.rs/serde_json/latest/serde_json/fn.to_vec.html "fn serde_json::to_vec") which serializes to a `Vec<u8>` and [`serde_json::to_writer`](https://docs.rs/serde_json/latest/serde_json/fn.to_writer.html "fn serde_json::to_writer") which serializes to any `io::Write` such as a File or a TCP stream.

```
use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
}

fn print_an_address() -> Result<()> {
    // Some data structure.
    let address = Address {
        street: "10 Downing Street".to_owned(),
        city: "London".to_owned(),
    };

    // Serialize it to a JSON string.
    let j = serde_json::to_string(&address)?;

    // Print, write to a file, or send to an HTTP server.
    println!("{}", j);

    Ok(())
}
```

Any type that implements Serde’s `Serialize` trait can be serialized this way. This includes built-in Rust standard library types like `Vec<T>` and `HashMap<K, V>`, as well as any structs or enums annotated with `#[derive(Serialize)]`.

## [§](https://docs.rs/serde_json/latest/serde_json/#no-std-support)No-std support

As long as there is a memory allocator, it is possible to use serde_json without the rest of the Rust standard library. Disable the default “std” feature and enable the “alloc” feature:

```
[dependencies]
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
```

For JSON support in Serde without a memory allocator, please see the [`serde-json-core`](https://github.com/rust-embedded-community/serde-json-core) crate.

[de](https://docs.rs/serde_json/latest/serde_json/de/index.html "mod serde_json::de")Deserialize JSON data to a Rust data structure.[error](https://docs.rs/serde_json/latest/serde_json/error/index.html "mod serde_json::error")When serializing or deserializing JSON goes wrong.[map](https://docs.rs/serde_json/latest/serde_json/map/index.html "mod serde_json::map")A map of String to serde*json::Value.[ser](https://docs.rs/serde_json/latest/serde_json/ser/index.html "mod serde_json::ser")`std`Serialize a Rust data structure into JSON data.[value](https://docs.rs/serde_json/latest/serde_json/value/index.html "mod serde_json::value")The Value enum, a loosely typed way of representing any valid JSON value.[json](https://docs.rs/serde_json/latest/serde_json/macro.json.html "macro serde_json::json")Construct a `serde_json::Value` from a JSON literal.[Deserializer](https://docs.rs/serde_json/latest/serde_json/struct.Deserializer.html "struct serde_json::Deserializer")A structure that deserializes JSON into Rust values.[Error](https://docs.rs/serde_json/latest/serde_json/struct.Error.html "struct serde_json::Error")This type represents all possible errors that can occur when serializing or deserializing JSON data.[Map](https://docs.rs/serde_json/latest/serde_json/struct.Map.html "struct serde_json::Map")Represents a JSON key/value type.[Number](https://docs.rs/serde_json/latest/serde_json/struct.Number.html "struct serde_json::Number")Represents a JSON number, whether integer or floating point.[Serializer](https://docs.rs/serde_json/latest/serde_json/struct.Serializer.html "struct serde_json::Serializer")`std`A structure for serializing Rust values into JSON.[Stream Deserializer](https://docs.rs/serde_json/latest/serde_json/struct.StreamDeserializer.html "struct serde_json::StreamDeserializer")Iterator that deserializes a stream into multiple JSON values.[Value](https://docs.rs/serde_json/latest/serde_json/enum.Value.html "enum serde_json::Value")Represents any valid JSON value.[from* reader](https://docs.rs/serde_json/latest/serde_json/fn.from_reader.html "fn serde*json::from_reader")`std`Deserialize an instance of type `T` from an I/O stream of JSON.[from* slice](https://docs.rs/serde_json/latest/serde_json/fn.from_slice.html "fn serde*json::from_slice")Deserialize an instance of type `T` from bytes of JSON text.[from* str](https://docs.rs/serde_json/latest/serde_json/fn.from_str.html "fn serde*json::from_str")Deserialize an instance of type `T` from a string of JSON text.[from* value](https://docs.rs/serde_json/latest/serde_json/fn.from_value.html "fn serde*json::from_value")Interpret a `serde_json::Value` as an instance of type `T`.[to* string](https://docs.rs/serde_json/latest/serde_json/fn.to_string.html "fn serde*json::to_string")`std`Serialize the given data structure as a String of JSON.[to* string* pretty](https://docs.rs/serde_json/latest/serde_json/fn.to_string_pretty.html "fn serde_json::to_string_pretty")`std`Serialize the given data structure as a pretty-printed String of JSON.[to* value](https://docs.rs/serde_json/latest/serde_json/fn.to_value.html "fn serde*json::to_value")Convert a `T` into `serde_json::Value` which is an enum that can represent any valid JSON data.[to_vec](https://docs.rs/serde_json/latest/serde_json/fn.to_vec.html "fn serde_json::to_vec")`std`Serialize the given data structure as a JSON byte vector.[to* vec* pretty](https://docs.rs/serde_json/latest/serde_json/fn.to_vec_pretty.html "fn serde_json::to_vec_pretty")`std`Serialize the given data structure as a pretty-printed JSON byte vector.[to* writer](https://docs.rs/serde_json/latest/serde_json/fn.to_writer.html "fn serde*json::to_writer")`std`Serialize the given data structure as JSON into the I/O stream.[to* writer\_ pretty](https://docs.rs/serde_json/latest/serde_json/fn.to_writer_pretty.html "fn serde_json::to_writer_pretty")`std`Serialize the given data structure as pretty-printed JSON into the I/O stream.[Result](https://docs.rs/serde_json/latest/serde_json/type.Result.html "type serde_json::Result")Alias for a `Result` with the error type `serde_json::Error`.
