use assert_matches::assert_matches;
use qi_format::{from_slice, Error, SliceDeserializer};
use serde::de::Deserializer;
use serde_value::{Value, ValueVisitor};

#[test]
fn test_slice_deserializer_deserialize_bool() {
    assert_matches!(from_slice::<bool>(&[0]), Ok(false));
    assert_matches!(from_slice::<bool>(&[1]), Ok(true));
    assert_matches!(from_slice::<bool>(&[2]), Err(Error::NotABoolValue(2)));
    assert_matches!(from_slice::<bool>(&[]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_i8() {
    assert_matches!(from_slice::<i8>(&[1]), Ok(1));
    assert_matches!(from_slice::<i8>(&[2]), Ok(2));
    assert_matches!(from_slice::<i8>(&[]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_u8() {
    assert_matches!(from_slice::<u8>(&[1]), Ok(1));
    assert_matches!(from_slice::<u8>(&[2]), Ok(2));
    assert_matches!(from_slice::<u8>(&[]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_i16() {
    assert_matches!(from_slice::<i16>(&[1, 0]), Ok(1));
    assert_matches!(from_slice::<i16>(&[2, 0]), Ok(2));
    assert_matches!(from_slice::<i16>(&[]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_u16() {
    assert_matches!(from_slice::<u16>(&[1, 0]), Ok(1));
    assert_matches!(from_slice::<u16>(&[2, 0]), Ok(2));
    assert_matches!(from_slice::<u16>(&[0]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_i32() {
    assert_matches!(from_slice::<i32>(&[1, 0, 0, 0]), Ok(1));
    assert_matches!(from_slice::<i32>(&[2, 0, 0, 0]), Ok(2));
    assert_matches!(from_slice::<i32>(&[0, 0, 0]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_u32() {
    assert_matches!(from_slice::<u32>(&[1, 0, 0, 0]), Ok(1));
    assert_matches!(from_slice::<u32>(&[2, 0, 0, 0]), Ok(2));
    assert_matches!(from_slice::<u32>(&[0, 0, 0]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_i64() {
    assert_matches!(from_slice::<i64>(&[1, 0, 0, 0, 0, 0, 0, 0]), Ok(1));
    assert_matches!(from_slice::<i64>(&[2, 0, 0, 0, 0, 0, 0, 0]), Ok(2));
    assert_matches!(
        from_slice::<i64>(&[0, 0, 0, 0, 0, 0, 0]),
        Err(Error::ShortRead)
    );
}

#[test]
fn test_slice_deserializer_deserialize_u64() {
    assert_matches!(from_slice::<u64>(&[1, 0, 0, 0, 0, 0, 0, 0]), Ok(1));
    assert_matches!(from_slice::<u64>(&[2, 0, 0, 0, 0, 0, 0, 0]), Ok(2));
    assert_matches!(
        from_slice::<u64>(&[0, 0, 0, 0, 0, 0, 0]),
        Err(Error::ShortRead)
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn test_slice_deserializer_deserialize_f32() {
    assert_matches!(from_slice::<f32>(&[0x14, 0xae, 0x29, 0x42]), Ok(42.42));
    assert_matches!(
        from_slice::<f32>(&[0xff, 0xff, 0xff, 0x7f]),
        Ok(f) => assert!(f.is_nan())
    );
    assert_matches!(from_slice::<f32>(&[0, 0, 0]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_f64() {
    assert_matches!(
        from_slice::<f64>(&[0xf6, 0x28, 0x5c, 0x8f, 0xc2, 0x35, 0x45, 0x40]),
        Ok(42.42)
    );
    assert_matches!(
        from_slice::<f64>(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]),
        Ok(f) => assert!(f.is_nan())
    );
    assert_matches!(
        from_slice::<f64>(&[0, 0, 0, 0, 0, 0, 0]),
        Err(Error::ShortRead)
    );
}

#[test]
fn test_slice_deserializer_deserialize_bytes() {
    assert_matches!(
        SliceDeserializer::new(&[4, 0, 0, 0, 1, 2, 3, 4]).deserialize_bytes(ValueVisitor),
        Ok(Value::Bytes(v)) => assert_eq!(v, [1, 2, 3, 4])
    );
    assert_matches!(
        SliceDeserializer::new(&[0, 0, 0, 0]).deserialize_bytes(ValueVisitor),
        Ok(Value::Bytes(v)) => assert!(v.is_empty())
    );
    assert_matches!(
        SliceDeserializer::new(&[4, 0, 0, 0, 1, 2, 3]).deserialize_bytes(ValueVisitor),
        Err(Error::ShortRead)
    );
    assert_matches!(
        SliceDeserializer::new(&[0, 0, 0]).deserialize_bytes(ValueVisitor),
        Err(Error::SequenceSize(_))
    );
}

#[test]
fn test_slice_deserializer_deserialize_option() {
    assert_matches!(from_slice::<Option::<i16>>(&[1, 42, 0]), Ok(Some(42i16)));
    assert_matches!(from_slice::<Option::<i16>>(&[0]), Ok(None));
    assert_matches!(from_slice::<Option::<i16>>(&[]), Err(Error::ShortRead));
}

#[test]
fn test_slice_deserializer_deserialize_unit() {
    assert_matches!(from_slice::<()>(&[]), Ok(()));
}

#[test]
fn test_slice_deserializer_deserialize_sequence() {
    assert_matches!(
        from_slice::<Vec::<i16>>(&[3, 0, 0, 0, 1, 0, 2, 0, 3, 0]),
        Ok(v) => assert_eq!(v, [1, 2, 3])
    );
    assert_matches!(
        from_slice::<Vec::<i16>>(&[0, 0, 0, 0]),
        Ok(v) => assert!(v.is_empty())
    );
    assert_matches!(
        from_slice::<Vec::<i16>>(&[1, 0, 0, 0]),
        Err(Error::SequenceElement { .. })
    );
    assert_matches!(
        from_slice::<Vec::<i16>>(&[0, 0, 0]),
        Err(Error::SequenceSize(_))
    );
}

#[test]
fn test_slice_deserializer_deserialize_tuple() {
    assert_matches!(
        from_slice::<(u32, Option<i8>)>(&[2, 0, 0, 0, 1, 2]),
        Ok((2, Some(2)))
    );
}

#[test]
fn test_slice_deserializer_deserialize_map() {
    use std::collections::HashMap;
    assert_matches!(
        from_slice::<HashMap::<i8, u8>>(&[2, 0, 0, 0, 1, 2, 2, 4]),
        Ok(m) => assert_eq!(m, HashMap::from([(1, 2), (2, 4)]))
    );
    assert_matches!(
        from_slice::<HashMap::<i8, u8>>(&[1, 0, 0, 0, 0]),
        Err(Error::MapValue { index: 0, .. })
    );
    assert_matches!(
        from_slice::<HashMap::<i8, u8>>(&[1, 0, 0, 0]),
        Err(Error::MapKey { index: 0, .. })
    );
    assert_matches!(
        from_slice::<HashMap::<i8, u8>>(&[0, 0, 0]),
        Err(Error::SequenceSize(_))
    );
}

// --------------------------------------------------------------
// Equivalence types
// --------------------------------------------------------------
#[test]
// char -> str
fn test_slice_deserializer_deserialize_char() {
    // `deserialize_char` yields strings, the visitor decides if it handles them or not.
    assert_matches!(from_slice::<char>(&[1, 0, 0, 0, 97]), Ok('a'));
    assert_matches!(from_slice::<char>(&[2, 0, 0, 0, 98, 99]), Ok('b'));
    assert_matches!(from_slice::<char>(&[1, 0, 0, 0]), Err(Error::ShortRead));
    assert_matches!(from_slice::<char>(&[0, 0, 0]), Err(Error::SequenceSize(_)));
}

#[test]
// str -> raw
fn test_slice_deserializer_deserialize_str() {
    assert_matches!(
        from_slice::<&str>(&[1, 0, 0, 0, 97]),
        Ok(s) => assert_eq!(s, "a")
    );
    assert_matches!(
        from_slice::<&str>(&[2, 0, 0, 0, 98, 99]),
        Ok(s) => assert_eq!(s, "bc")
    );
    assert_matches!(from_slice::<&str>(&[1, 0, 0, 0]), Err(Error::ShortRead));
    assert_matches!(from_slice::<&str>(&[0, 0, 0]), Err(Error::SequenceSize(_)));
}

#[test]
fn test_slice_deserializer_deserialize_string() {
    assert_matches!(
        from_slice::<String>(&[1, 0, 0, 0, 97]),
        Ok(s) => assert_eq!(s, "a")
    );
    assert_matches!(
        from_slice::<String>(&[2, 0, 0, 0, 98, 99]),
        Ok(s) => assert_eq!(s, "bc")
    );
    assert_matches!(
        from_slice::<String>(&[0, 0, 0, 0]),
        Ok(s) => assert!(s.is_empty())
    );
    assert_matches!(from_slice::<String>(&[1, 0, 0, 0]), Err(Error::ShortRead));
    assert_matches!(
        from_slice::<String>(&[0, 0, 0]),
        Err(Error::SequenceSize(_))
    );
}

#[test]
fn test_slice_deserializer_deserialize_byte_buf() {
    assert_matches!(
        SliceDeserializer::new(&[1, 0, 0, 0, 97]).deserialize_byte_buf(ValueVisitor),
        Ok(Value::Bytes(b)) => assert_eq!(b, [97])
    );
    assert_matches!(
        SliceDeserializer::new(&[2, 0, 0, 0, 98, 99]).deserialize_byte_buf(ValueVisitor),
        Ok(Value::Bytes(b)) => assert_eq!(b, [98, 99])
    );
    assert_matches!(
        SliceDeserializer::new(&[0, 0, 0, 0]).deserialize_byte_buf(ValueVisitor),
        Ok(Value::Bytes(b)) => assert!(b.is_empty())
    );
    assert_matches!(
        SliceDeserializer::new(&[1, 0, 0, 0]).deserialize_byte_buf(ValueVisitor),
        Err(Error::ShortRead)
    );
    assert_matches!(
        SliceDeserializer::new(&[0, 0, 0]).deserialize_byte_buf(ValueVisitor),
        Err(Error::SequenceSize(_))
    );
}

#[test]
// struct(T...) -> tuple(T...)
fn test_slice_deserializer_deserialize_struct() {
    #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
    struct S {
        c: char,
        s: std::string::String,
        // t: (u8, i8, i16),
        // i: i32,
    }
    assert_matches!(
        from_slice::<S>(&[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99]),
        Ok(S { c: 'a', s}) => assert_eq!(s, "bc")
    );
    // assert_matches!(
    //     from_slice::<S>(&[1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99, 0, 0, 0, 0, 3, 0, 0, 0]),
    //     Ok(S {
    //         c: 'a',
    //         s,
    //         t: (0, 0, 0),
    //         i: 3
    //     }) => assert_eq!(s, "bc")
    // );
}

#[test]
// newtype_struct(T) -> tuple(T) = T
fn test_slice_deserializer_deserialize_newtype_struct() {
    #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
    struct S(char);
    assert_matches!(from_slice::<S>(&[1, 0, 0, 0, 97]), Ok(S('a')));
    assert_matches!(from_slice::<S>(&[1, 0, 0, 0, 98]), Ok(S('b')));
}

#[test]
// unit_struct -> unit
fn test_slice_deserializer_deserialize_unit_struct() {
    #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
    struct S;
    assert_matches!(from_slice::<S>(&[]), Ok(S));
}

#[test]
// tuple_struct(T...) -> tuple(T...)
fn test_slice_deserializer_deserialize_tuple_struct() {
    #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
    struct S(String, Vec<i16>);
    assert_matches!(
        from_slice::<S>(&[1, 0, 0, 0, 97, 3, 0, 0, 0, 4, 0, 5, 0, 6, 0]),
        Ok(S(str, v)) => {
            assert_eq!(str, "a");
            assert_eq!(v, [4, 5, 6])
        }
    );
}

#[test]
// enum(idx,T) -> tuple(idx,T)
fn test_slice_deserializer_deserialize_enum() {
    #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
    enum E {
        A,
        B(char),
        C,
        D(i16, i16, i16),
    }
    assert_matches!(from_slice::<E>(&[0, 0, 0, 0]), Ok(E::A));
    assert_matches!(
        from_slice::<E>(&[1, 0, 0, 0, 1, 0, 0, 0, 97]),
        Ok(E::B('a'))
    );
    assert_matches!(
        from_slice::<E>(&[3, 0, 0, 0, 4, 0, 5, 0, 6, 0]),
        Ok(E::D(4, 5, 6))
    );
}

#[test]
// identifier => unit
fn test_slice_deserializer_deserialize_identifier() {
    assert_matches!(
        SliceDeserializer::new(&[]).deserialize_identifier(ValueVisitor),
        Ok(Value::Unit)
    );
}

// --------------------------------------------------------------
// Unhandled types
// --------------------------------------------------------------
#[test]
fn test_slice_deserializer_deserialize_i128() {
    assert_matches!(
        SliceDeserializer::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
            .deserialize_i128(ValueVisitor),
        Err(Error::Custom(_))
    );
}

#[test]
fn test_slice_deserializer_deserialize_u128() {
    assert_matches!(
        SliceDeserializer::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
            .deserialize_u128(ValueVisitor),
        Err(Error::Custom(_))
    );
}

#[test]
fn test_slice_deserializer_deserialize_any() {
    assert_matches!(
        SliceDeserializer::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
            .deserialize_any(ValueVisitor),
        Err(Error::UnknownElement)
    );
}

#[test]
fn test_slice_deserializer_deserialize_ignored_any() {
    assert_matches!(
        SliceDeserializer::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
            .deserialize_any(ValueVisitor),
        Err(Error::UnknownElement)
    );
}
