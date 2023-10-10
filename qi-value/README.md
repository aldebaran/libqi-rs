# qi-value

Values of the `qi` type system as defined in the `spec:/aldebaran/framework/2022/h` specification.

## Minimum Rust Required Version (MSRV)

This crate requires Rust 1.63+.

## Mapping between `serde` and `qi` type systems

The following table defines the mapping between `serde` types and `qi` types:

| `serde` | `qi` |
| - | - |
| `unit` | `unit` |
| `bool` | `bool` |
| `i8` | `int_8` |
| `u8` | `uint_8` |
| `i16` | `int_16` |
| `u16` | `uint_16` |
| `i32` | `int_32` |
| `u32` | `uint_32` |
| `i64` | `int_64` |
| `u64` | `uint_64` |
| `f32` | `float_32` |
| `f64` | `float_64` |
| `char` | `string` of a single element |
| `str` | `string` |
| `bytes` | `raw` |
| `option` | `optional` |
| `sequence(T)` | `list(T)` |
| `tuple(T...)` | `tuple(T...)` |
| `map(T,U)` | `map(T,U)` |
| `struct(T...)` | `tuple(T...)` |
| `newtype_struct(T)` | `tuple(T)` = `T` |
| `unit_struct` | `unit` |
| `tuple_struct(T...)` | `tuple(T...)` |
| `enum(idx,T)` | `tuple(idx: uint_32, T)` |

The following table defines the mapping from `qi` types to `serde` types:

| `qi` | `serde` |
| - | - |
| `unit` | `unit` |
| `bool` | `bool` |
| `int_8` | `i8` |
| `uint_8` | `u8` |
| `int_16` | `i16` |
| `uint_16` | `u16` |
| `int_32` | `i32` |
| `uint_32` | `u32` |
| `int_64` | `i64` |
| `uint_64` | `u64` |
| `float_32` | `f32` |
| `float_64` | `f64` |
| `string` | `str / string` |
| `raw` | `bytes / byte_buf` |
| `optional` | `option` |
| `list(T)` | `sequence(T)` |
| `map(T,U)` | `map(T,U)` |
| `tuple(T...)` | `tuple(T...)` |

The following table defines how some `qi` types are interpreted as other `qi` types:

| `qi` | interpreted as |
| - | - |
| `signature` | `string` |
| `object` | `tuple(metaobject, uint_32, uint_32, tuple(uint_32, uint_32, uint_32, uint_32, uint_32))` |
| `metaobject` | `tuple(map(uint_32, metamethod), map(uint_32, metainfo), map(uint_32, metainfo), string)` |
| `metamethod` | `tuple(uint_32, signature, string, signature, string, map(string, string), string)` |
| `metaproperty` | `tuple(uint_32, string, signature)` |
| `metasignal` | `tuple(uint_32, string, signature)` |
| `dynamic(T)` | `tuple(signature, T)` |
