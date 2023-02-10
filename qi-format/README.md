The `qi-format` crate exposes types and functions for manipulating values from
the `qi` type system and to serialize and deserialize values of these types from
data in the `qi` format.

The `qi` format is a binary representation of values. It is mainly used for
communicating values in the `qi` messaging protocol.

# Minimum Rust Required Version (MSRV)

This crate requires Rust 1.59+.

# Getting started

## Serializing a type into the format

TODO

## Serializing a type into a `Value`

TODO

## Deserializing a type from the format

TODO

## Deserializing a type from a value

TODO

# `qi` type system and format

You may refer to the `qi` type system and format specification (reference
`spec:/aldebaran/framework/2022/h`) document for details.

# Implementation of the format

Let `Qi` be the type of all values representable in the `qi` format (equivalent
to the `qi` type system).

The type [`Serializer`] implements [`serde::ser::Serializer`] and handles
serialization of any serializable type into the `qi` format.

The type [`Deserializer`] implements [`serde::de::Deserializer`] and handles deserialization
of any deserializable type from the `qi` format.

The following table defines the bijection between `serde` types and `qi` types.

| `serde` | `qi` |
| - | - |
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
| `bytes` | `raw` |
| `option` | `optional` |
| `unit` | `unit` |
| `sequence(T)` [^list-known-size] | `list(T)` |
| `tuple(T...)` | `tuple(T...)` |
| `map(T,U)` | `map(T, U)` |

For other `serde` and `qi` types not present in this set, we defined an
associated type, that will be used instead as representation / interpretation.
For some of these, the conversion is not isomorphic (e.g. between strings and
bytes). Any conversion error will result in an error in serialization or
deserialization.

| `serde` | associated `serde` type |
| - | - |
| `char` | `str` of a single element[^string-as-bytes] |
| `str` [^de-only] | `bytes` [^string-as-bytes] |
| `string` | `bytes` [^string-as-bytes] |
| `byte_buf` [^de-only] | `bytes` |
| `struct(T...)` | `tuple(T...)` |
| `newtype_struct(T)` | `tuple(T)` = `T` |
| `unit_struct` | `unit` |
| `tuple_struct(T...)` | `tuple(T...)` |
| `unit_variant(idx)` [^ser-only] | `tuple(idx: uint_32, unit)` |
| `newtype_variant(idx,T)` [^ser-only] | `tuple(idx: uint_32, tuple(T))` |
| `tuple_variant(idx,T...)` [^ser-only] | `tuple(idx: uint_32, tuple(T...))` |
| `struct_variant(idx,T...)` [^ser-only] | `tuple(idx: uint_32, tuple(T...))` |
| `enum(idx,T)` [^de-only] | `tuple(idx: uint_32,T)` |
| `identifier` [^de-only] | `unit` [^no-ident] |

| `qi` | associated `qi` type |
| - | - |
| `string` | `raw` [^string-as-bytes] |
| `signature` | `raw` [^string-as-bytes] |
| `object` | `tuple(metaobject, uint_32, uint_32, tuple(uint_32, uint_32, uint_32, uint_32, uint_32))` |
| `metaobject` | `tuple(map(uint_32, metamethod), map(uint_32, metainfo), map(uint_32, metainfo), string)` |
| `metamethod` | `tuple(metainfo, string, string, map(string, string), string)` |
| `metainfo` | `tuple(uint_32, string, string)` |
| `dynamic(T)` | `tuple(signature, T)` |

The following `serde` types are not handled (i.e. their serialization or
deserialization with the `qi` serializer / deserializer will always result in errors):

- `i128`
- `u128`
- `any` [^de-only]
- `ignored any` [^de-only]

[^ser-only]: serialization only.

[^de-only]: deserialization only.

[^string-as-bytes]: `qi` strings encoding is not specified, whereas strings in Rust must be
UTF-8. Therefore `qi` strings are mapped to byte arrays.

[^no-ident]: identifiers are not serialized in the `qi` format.

[^list-known-size]: The size of the sequence must be known when serialized, otherwise an error occurs.

## Other types

The crate defines a Rust type (or at least a type alias) for each `qi` type, and more.
Each of those types implements [`serde::Serialize`] and [`serde::Deserialize`], which
means they can be represented into and interpreted from the format.
