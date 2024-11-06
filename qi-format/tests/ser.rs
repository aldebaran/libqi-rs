use assert_matches::assert_matches;
use qi_format::{BytesSerializer, Error};
use serde::ser::Serializer;

// --------------------------------------------------------------
// Bijection types
// --------------------------------------------------------------

#[test]
fn test_bytes_serializer_serialize_bool() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_bool(true).unwrap();
    assert_eq!(*bytes, [1]);

    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_bool(false).unwrap();
    assert_eq!(*bytes, [0]);
}

#[test]
fn test_bytes_serializer_serialize_i8() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_i8(42).unwrap();
    assert_eq!(*bytes, [42]);
}

#[test]
fn test_bytes_serializer_serialize_u8() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_u8(42).unwrap();
    assert_eq!(*bytes, [42]);
}

#[test]
fn test_bytes_serializer_serialize_i16() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_i16(42).unwrap();
    assert_eq!(*bytes, [42, 0]);
}

#[test]
fn test_bytes_serializer_serialize_u16() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_u16(42).unwrap();
    assert_eq!(*bytes, [42, 0]);
}

#[test]
fn test_bytes_serializer_serialize_i32() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_i32(42).unwrap();
    assert_eq!(*bytes, [42, 0, 0, 0]);
}

#[test]
fn test_bytes_serializer_serialize_u32() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_u32(42).unwrap();
    assert_eq!(*bytes, [42, 0, 0, 0]);
}

#[test]
fn test_bytes_serializer_serialize_i64() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_i64(42).unwrap();
    assert_eq!(*bytes, [42, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_bytes_serializer_serialize_u64() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_u64(42).unwrap();
    assert_eq!(*bytes, [42, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_bytes_serializer_serialize_f32() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_f32(1.0).unwrap();
    assert_eq!(*bytes, [0, 0, 128, 63]);
}

#[test]
fn test_bytes_serializer_serialize_f64() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_f64(1.0).unwrap();
    assert_eq!(*bytes, [0, 0, 0, 0, 0, 0, 240, 63]);
}

#[test]
fn test_bytes_serializer_serialize_bytes() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_bytes(&[1, 2, 3, 4, 5]).unwrap();
    assert_eq!(*bytes, [5, 0, 0, 0, 1, 2, 3, 4, 5]);
}

#[test]
fn test_bytes_serializer_serialize_option() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_some(&42i16).unwrap();
    assert_eq!(*bytes, [1, 42, 0]);

    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_none().unwrap();
    assert_eq!(*bytes, [0]);
}

#[test]
fn test_bytes_serializer_serialize_unit() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_unit().unwrap();
    assert_eq!(*bytes, []);
}

#[test]
fn test_bytes_serializer_serialize_sequence() {
    let serializer = BytesSerializer::default();
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(3)).unwrap();
    seq.serialize_element(&12i16).unwrap();
    seq.serialize_element(&22i16).unwrap();
    seq.serialize_element(&23i16).unwrap();
    // More elements result in error.
    assert_matches!(seq.serialize_element(&3), Err(Error::UnexpectedElement(3)));
    let bytes = seq.end().unwrap();
    assert_eq!(*bytes, [3, 0, 0, 0, 12, 0, 22, 0, 23, 0]);
}

#[test]
fn test_bytes_serializer_serialize_sequence_unknown_size() {
    let serializer = BytesSerializer::default();
    assert_matches!(
        serializer.serialize_seq(None),
        Err(Error::MissingSequenceSize)
    );
}

#[test]
fn test_bytes_serializer_serialize_tuple() {
    let serializer = BytesSerializer::default();
    use serde::ser::SerializeTuple;
    let mut tuple = serializer.serialize_tuple(2).unwrap();
    tuple.serialize_element(&42i16).unwrap();
    tuple.serialize_element(&true).unwrap();
    // More elements result in error.
    assert_matches!(
        tuple.serialize_element(&1290u32),
        Err(Error::UnexpectedElement(2))
    );
    let bytes = tuple.end().unwrap();
    assert_eq!(*bytes, [42, 0, 1]);
}

#[test]
fn test_bytes_serializer_serialize_map() {
    let serializer = BytesSerializer::default();
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(2)).unwrap();
    map.serialize_entry(&31, &false).unwrap();
    map.serialize_entry(&64, &true).unwrap();
    // More elements result in error.
    assert_matches!(
        map.serialize_entry(&123u8, &246u16),
        Err(Error::UnexpectedElement(2))
    );
    let bytes = map.end().unwrap();
    assert_eq!(*bytes, [2, 0, 0, 0, 31, 0, 0, 0, 0, 64, 0, 0, 0, 1]);
}

#[test]
fn test_bytes_serializer_serialize_map_unknown_size() {
    let serializer = BytesSerializer::default();
    assert_matches!(
        serializer.serialize_map(None),
        Err(Error::MissingSequenceSize)
    );
}

// --------------------------------------------------------------
// Equivalence types
// --------------------------------------------------------------

#[test]
// char -> str
fn test_bytes_serializer_serialize_char() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_char('a').unwrap();
    assert_eq!(*bytes, [1, 0, 0, 0, 97]);
}

#[test]
// str -> bytes.
fn test_bytes_serializer_serialize_str() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_str("abc").unwrap();
    assert_eq!(*bytes, [3, 0, 0, 0, 97, 98, 99]);
}

#[test]
// struct -> tuple.
fn test_bytes_serializer_serialize_struct() {
    let serializer = BytesSerializer::default();
    use serde::ser::SerializeStruct;
    let mut st = serializer.serialize_struct("MyStruct", 2).unwrap();
    st.serialize_field("w", &23u16).unwrap();
    st.serialize_field("c", &'a').unwrap();
    // More fields result in errors.
    assert_matches!(
        st.serialize_field("c", &'a'),
        Err(Error::UnexpectedElement(2))
    );
    let bytes = st.end().unwrap();
    assert_eq!(*bytes, [23, 0, 1, 0, 0, 0, 97]);
}

#[test]
// newtype_struct -> tuple(T) = T
fn test_bytes_serializer_serialize_newtype_struct() {
    let serializer = BytesSerializer::default();
    let bytes = serializer
        .serialize_newtype_struct("MyStruct", &12i32)
        .unwrap();
    assert_eq!(*bytes, [12, 0, 0, 0]);
}

#[test]
// unit_struct -> unit
fn test_bytes_serializer_serialize_unit_struct() {
    let serializer = BytesSerializer::default();
    let bytes = serializer.serialize_unit_struct("MyStruct").unwrap();
    assert_eq!(*bytes, []);
}

#[test]
// tuple_struct -> tuple
fn test_bytes_serializer_serialize_tuple_struct() {
    let serializer = BytesSerializer::default();
    use serde::ser::SerializeTupleStruct;
    let mut tuple = serializer.serialize_tuple_struct("MyStruct", 2).unwrap();
    tuple.serialize_field(&234u16).unwrap();
    tuple.serialize_field(&'a').unwrap();
    // More elements result in error.
    assert_matches!(
        tuple.serialize_field(&true),
        Err(Error::UnexpectedElement(2))
    );
    let bytes = tuple.end().unwrap();
    assert_eq!(*bytes, [234, 0, 1, 0, 0, 0, 97]);
}

#[test]
// unit_variant -> tuple(uint32) = uint32
fn test_bytes_serializer_serialize_unit_variant() {
    let serializer = BytesSerializer::default();
    let bytes = serializer
        .serialize_unit_variant("MyEnum", 23, "MyVariant")
        .unwrap();
    assert_eq!(*bytes, [23, 0, 0, 0])
}

#[test]
// newtype_variant(T) -> tuple(uint32, T)
fn test_bytes_serializer_serialize_newtype_variant() {
    let serializer = BytesSerializer::default();
    let bytes = serializer
        .serialize_newtype_variant("MyEnum", 123, "MyVariant", "abc")
        .unwrap();
    assert_eq!(*bytes, [123, 0, 0, 0, 3, 0, 0, 0, 97, 98, 99]);
}

#[test]
// tuple_variant(T...) -> tuple(uint32, tuple(T...)) = tuple(uint32, T...)
fn test_bytes_serializer_serialize_tuple_variant() {
    let serializer = BytesSerializer::default();
    use serde::ser::SerializeTupleVariant;
    let mut tuple = serializer
        .serialize_tuple_variant("MyEnum", 913, "MyVariant", 2)
        .unwrap();
    tuple.serialize_field(&3290u16).unwrap();
    tuple.serialize_field("def").unwrap();
    // More elements result in error.
    assert_matches!(
        tuple.serialize_field(&true),
        Err(Error::UnexpectedElement(2))
    );
    let bytes = tuple.end().unwrap();
    assert_eq!(*bytes, [145, 3, 0, 0, 218, 12, 3, 0, 0, 0, 100, 101, 102]);
}

#[test]
// struct_variant(T...) -> tuple(uint32, tuple(T...)) = tuple(uint32, T...)
fn test_bytes_serializer_serialize_struct_variant() {
    let serializer = BytesSerializer::default();
    use serde::ser::SerializeStructVariant;
    let mut st = serializer
        .serialize_struct_variant("MyEnum", 128, "MyVariant", 3)
        .unwrap();
    st.serialize_field("t", &(1u8, 2u8)).unwrap();
    st.serialize_field("c", &'1').unwrap();
    st.serialize_field("b", &true).unwrap();
    // More elements result in error.
    assert_matches!(
        st.serialize_field("s", "abc"),
        Err(Error::UnexpectedElement(3))
    );
    let bytes = st.end().unwrap();
    assert_eq!(*bytes, [128, 0, 0, 0, 1, 2, 1, 0, 0, 0, 49, 1]);
}
