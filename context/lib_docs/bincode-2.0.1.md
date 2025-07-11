Title: bincode - Rust

URL Source: https://docs.rs/bincode/latest/bincode/

Markdown Content:
Expand description

Bincode is a crate for encoding and decoding using a tiny binary serialization strategy. Using it, you can easily go from having an object in memory, quickly serialize it to bytes, and then deserialize it back just as fast!

If you’re coming from bincode 1, check out our [migration guide](https://docs.rs/bincode/latest/bincode/migration_guide/index.html)

## [§](https://docs.rs/bincode/latest/bincode/#serde)Serde

Starting from bincode 2, serde is now an optional dependency. If you want to use serde, please enable the `serde` feature. See [Features](https://docs.rs/bincode/latest/bincode/#features) for more information.

## [§](https://docs.rs/bincode/latest/bincode/#features)Features

| Name   | Default? | Affects MSRV?               | Supported types for Encode/Decode                                                        | Enabled methods                                                                                                                            | Other                                                                                                                                     |
| ------ | -------- | --------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| std    | Yes      | No                          | `HashMap` and `HashSet`                                                                  | `decode_from_std_read` and `encode_into_std_write`                                                                                         |                                                                                                                                           |
| alloc  | Yes      | No                          | All common containers in alloc, like `Vec`, `String`, `Box`                              | `encode_to_vec`                                                                                                                            |                                                                                                                                           |
| atomic | Yes      | No                          | All `Atomic*` integer types, e.g. `AtomicUsize`, and `AtomicBool`                        |                                                                                                                                            |                                                                                                                                           |
| derive | Yes      | No                          |                                                                                          |                                                                                                                                            | Enables the `BorrowDecode`, `Decode` and `Encode` derive macros                                                                           |
| serde  | No       | Yes (MSRV reliant on serde) | `Compat` and `BorrowCompat`, which will work for all types that implement serde’s traits | serde-specific encode/decode functions in the [serde](https://docs.rs/bincode/latest/bincode/serde/index.html "mod bincode::serde") module | Note: There are several [known issues](https://docs.rs/bincode/latest/bincode/serde/index.html#known-issues) when using serde and bincode |

## [§](https://docs.rs/bincode/latest/bincode/#which-functions-to-use)Which functions to use

Bincode has a couple of pairs of functions that are used in different situations.

| Situation                                                                                                                                                                                                                                                  | Encode                                                                                                                              | Decode                                                                                                                           |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| You’re working with [`fs::File`](https://doc.rust-lang.org/nightly/std/fs/struct.File.html "struct std::fs::File") or [`net::TcpStream`](https://doc.rust-lang.org/nightly/std/net/tcp/struct.TcpStream.html "struct std::net::tcp::TcpStream")            | [`encode_into_std_write`](https://docs.rs/bincode/latest/bincode/fn.encode_into_std_write.html "fn bincode::encode_into_std_write") | [`decode_from_std_read`](https://docs.rs/bincode/latest/bincode/fn.decode_from_std_read.html "fn bincode::decode_from_std_read") |
| you’re working with in-memory buffers                                                                                                                                                                                                                      | [`encode_to_vec`](https://docs.rs/bincode/latest/bincode/fn.encode_to_vec.html "fn bincode::encode_to_vec")                         | [`decode_from_slice`](https://docs.rs/bincode/latest/bincode/fn.decode_from_slice.html "fn bincode::decode_from_slice")          |
| You want to use a custom [Reader](https://docs.rs/bincode/latest/bincode/de/read/trait.Reader.html "trait bincode::de::read::Reader") and [Writer](https://docs.rs/bincode/latest/bincode/enc/write/trait.Writer.html "trait bincode::enc::write::Writer") | [`encode_into_writer`](https://docs.rs/bincode/latest/bincode/fn.encode_into_writer.html "fn bincode::encode_into_writer")          | [`decode_from_reader`](https://docs.rs/bincode/latest/bincode/fn.decode_from_reader.html "fn bincode::decode_from_reader")       |
| You’re working with pre-allocated buffers or on embedded targets                                                                                                                                                                                           | [`encode_into_slice`](https://docs.rs/bincode/latest/bincode/fn.encode_into_slice.html "fn bincode::encode_into_slice")             | [`decode_from_slice`](https://docs.rs/bincode/latest/bincode/fn.decode_from_slice.html "fn bincode::decode_from_slice")          |

**Note:** If you’re using `serde`, use `bincode::serde::...` instead of `bincode::...`

## [§](https://docs.rs/bincode/latest/bincode/#example)Example

```
let mut slice = [0u8; 100];

// You can encode any type that implements `Encode`.
// You can automatically implement this trait on custom types with the `derive` feature.
let input = (
    0u8,
    10u32,
    10000i128,
    'a',
    [0u8, 1u8, 2u8, 3u8]
);

let length = bincode::encode_into_slice(
    input,
    &mut slice,
    bincode::config::standard()
).unwrap();

let slice = &slice[..length];
println!("Bytes written: {:?}", slice);

// Decoding works the same as encoding.
// The trait used is `Decode`, and can also be automatically implemented with the `derive` feature.
let decoded: (u8, u32, i128, char, [u8; 4]) = bincode::decode_from_slice(slice, bincode::config::standard()).unwrap().0;

assert_eq!(decoded, input);
```

` pub use de::BorrowDecode;``pub use de::Decode;``pub use enc::Encode; `[config](https://docs.rs/bincode/latest/bincode/config/index.html "mod bincode::config")The config module is used to change the behavior of bincode’s encoding and decoding logic.[de](https://docs.rs/bincode/latest/bincode/de/index.html "mod bincode::de")Decoder-based structs and traits.[enc](https://docs.rs/bincode/latest/bincode/enc/index.html "mod bincode::enc")Encoder-based structs and traits.[error](https://docs.rs/bincode/latest/bincode/error/index.html "mod bincode::error")Errors that can be encounting by Encoding and Decoding.[migration\_ guide](https://docs.rs/bincode/latest/bincode/migration_guide/index.html "mod bincode::migration_guide")Migrating from bincode 1 to 2[serde](https://docs.rs/bincode/latest/bincode/serde/index.html "mod bincode::serde")`serde`Support for serde integration. Enable this with the `serde` feature.[spec](https://docs.rs/bincode/latest/bincode/spec/index.html "mod bincode::spec")Serialization Specification[impl* borrow* decode](https://docs.rs/bincode/latest/bincode/macro.impl_borrow_decode.html "macro bincode::impl_borrow_decode")Helper macro to implement `BorrowDecode` for any type that implements `Decode`.[impl* borrow* decode* with* context](https://docs.rs/bincode/latest/bincode/macro.impl_borrow_decode_with_context.html "macro bincode::impl_borrow_decode_with_context")Helper macro to implement `BorrowDecode` for any type that implements `Decode`.[borrow* decode* from\_ slice](https://docs.rs/bincode/latest/bincode/fn.borrow_decode_from_slice.html "fn bincode::borrow_decode_from_slice")Attempt to decode a given type `D` from the given slice. Returns the decoded output and the amount of bytes read.[borrow* decode* from* slice* with\_ context](https://docs.rs/bincode/latest/bincode/fn.borrow_decode_from_slice_with_context.html "fn bincode::borrow_decode_from_slice_with_context")Attempt to decode a given type `D` from the given slice with `Context`. Returns the decoded output and the amount of bytes read.[decode* from* reader](https://docs.rs/bincode/latest/bincode/fn.decode_from_reader.html "fn bincode::decode_from_reader")Attempt to decode a given type `D` from the given [Reader](https://docs.rs/bincode/latest/bincode/de/read/trait.Reader.html "trait bincode::de::read::Reader").[decode* from* slice](https://docs.rs/bincode/latest/bincode/fn.decode_from_slice.html "fn bincode::decode_from_slice")Attempt to decode a given type `D` from the given slice. Returns the decoded output and the amount of bytes read.[decode* from* slice* with* context](https://docs.rs/bincode/latest/bincode/fn.decode_from_slice_with_context.html "fn bincode::decode_from_slice_with_context")Attempt to decode a given type `D` from the given slice with `Context`. Returns the decoded output and the amount of bytes read.[decode* from* std\_ read](https://docs.rs/bincode/latest/bincode/fn.decode_from_std_read.html "fn bincode::decode_from_std_read")`std`Decode type `D` from the given reader with the given `Config`. The reader can be any type that implements `std::io::Read`, e.g. `std::fs::File`.[decode* from* std* read* with\_ context](https://docs.rs/bincode/latest/bincode/fn.decode_from_std_read_with_context.html "fn bincode::decode_from_std_read_with_context")`std`Decode type `D` from the given reader with the given `Config` and `Context`. The reader can be any type that implements `std::io::Read`, e.g. `std::fs::File`.[encode* into* slice](https://docs.rs/bincode/latest/bincode/fn.encode_into_slice.html "fn bincode::encode_into_slice")Encode the given value into the given slice. Returns the amount of bytes that have been written.[encode* into* std\_ write](https://docs.rs/bincode/latest/bincode/fn.encode_into_std_write.html "fn bincode::encode_into_std_write")`std`Encode the given value into any type that implements `std::io::Write`, e.g. `std::fs::File`, with the given `Config`. See the [config](https://docs.rs/bincode/latest/bincode/config/index.html) module for more information. Returns the amount of bytes written.[encode* into* writer](https://docs.rs/bincode/latest/bincode/fn.encode_into_writer.html "fn bincode::encode_into_writer")Encode the given value into a custom [Writer](https://docs.rs/bincode/latest/bincode/enc/write/trait.Writer.html "trait bincode::enc::write::Writer").[encode* to* vec](https://docs.rs/bincode/latest/bincode/fn.encode_to_vec.html "fn bincode::encode_to_vec")`alloc`Encode the given value into a `Vec<u8>` with the given `Config`. See the [config](https://docs.rs/bincode/latest/bincode/config/index.html) module for more information.[Borrow Decode](https://docs.rs/bincode/latest/bincode/derive.BorrowDecode.html "derive bincode::BorrowDecode")`derive`[Decode](https://docs.rs/bincode/latest/bincode/derive.Decode.html "derive bincode::Decode")`derive`[Encode](https://docs.rs/bincode/latest/bincode/derive.Encode.html "derive bincode::Encode")`derive`
