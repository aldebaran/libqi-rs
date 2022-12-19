The `qi-format` crate exposes types and functions for manipulating values from
the `qi` type system and to serialize and deserialize values of these types from
data in the `qi` format.

The `qi` format is a binary representation of values. It is mainly used for
communicating values in the `qi` messaging protocol.

# Minimum Rust Required Version (MSRV)

This crate requires Rust 1.54+.

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

## Definition of serialization

Let `T` be a type and `F` a format type, i.e. the set of all values
representable by the format.

`T` is serializable into `F` if there is a function: `serialize: T -> F`.

`T` is deserializable from `F` if there is a function: `deserialize: F -> T`.

Additionally, to ensure the consistency of the meaning of types between the
serializer and the deserializer, each of these functions must be bijections and
the inverse of the other.

This means that, assuming that values of `T` and `F` are equality-comparable,
for `t: T` and `f: F`:

`deserialize(serialize(t)) == t && serialize(deserialize(f)) == f`

This also means that the meaning of a type is preserved through serialization
in a format.

## `serde`

Each type that implements [`serde::Serialize`] and/or [`serde::Deserialize`]
defines its representation in the Serde data model.

Serializers and deserializers types that implement respectively
[`serde::ser::Serializer`] and
[`serde::de::Deserializer`] then define an association
between the Serde data model and the types of their format.

We denote the following types:

- `Serde` be the sum of the types of all values in the `serde` data model.
- `SerdeKind` be the kind of the types of values in the `serde` data model.
- `Qi` to be a format type.

You may refer to the documentation of the `serde` crate for details on the
types of the data model.

## `Serializer` and `Deserializer` for the `qi` format

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
| `char` | `bytes` of a single element[^string-as-bytes] |
| `str` [^de-only] | `bytes` [^string-as-bytes] |
| `string` | `bytes` [^string-as-bytes] |
| `byte_buf` [^de-only] | `bytes` |
| `struct(T...)` | `tuple(T...)` |
| `newtype_struct(T)` | `tuple(T)` |
| `unit_struct` | `unit` |
| `tuple_struct(T...)` | `tuple(T...)` |
| `unit_variant` [^ser-only] | `tuple(uint_32, unit)` where the first element is the variant index |
| `newtype_variant(T)` [^ser-only] | `tuple(uint_32, T)` where the first element is the variant index |
| `tuple_variant(T...)` [^ser-only] | `tuple(uint_32, tuple(T...))` where the first element is the variant index |
| `struct_variant(T...)` [^ser-only] | `tuple(uint_32, tuple(T...))` where the first element is the variant index |
| `enum` [^de-only] | `tuple(uint_32,T)` where the first element is the variant index and the second is the associated value |
| `identifier` [^de-only] | `unit` [^no-ident] |

| `qi` | associated `qi` type |
| - | - |
| `string` | `raw` [^string-as-bytes] |
| `signature` | `raw` [^string-as-bytes] |
| `object` | `tuple(metaobject, uint_32, uint_32, tuple(uint_32, uint_32, uint_32, uint_32, uint_32))` |
| `metaobject` | `tuple(map(uint_32, metamethod), map(uint_32, metainfo), map(uint_32, metainfo), string)` |
| `metamethod` | `tuple(metainfo, string, string, map(string, string), string)` |
| `metainfo` | `tuple(uint_32, string, string)` |
| `annotated(T)` | `tuple(signature, T)` |

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

### Value

The [`Value`] enumeration, that represents any value of the `qi` format,
implements [`serde::ser::Serializer`] and [`serde::de::Deserializer`] (with
[`serde::de::IntoDeserializer`]).

This means that:

- any type that implements [`serde::Serialize`] can be serialized
into a `Value` through the [`to_value`] function,
- any type that implements [`serde::Deserialize`]
can be deserialized from a `Value` through the [`from_value`] (or
[`from_value_owned`]) function (assuming the value is of the right type).
