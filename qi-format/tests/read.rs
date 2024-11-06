use assert_matches::assert_matches;
use bytes::Bytes;
use qi_format::{
    read::{
        read_bool, read_f32, read_f64, read_i16, read_i32, read_i64, read_i8, read_raw, read_size,
        read_string, read_u16, read_u32, read_u64, read_u8, StringOrByteBuf,
    },
    Error,
};

#[test]
fn test_slice_read_string() {
    let mut buf = &[1, 0, 0, 0, 100, 4, 0, 0, 0, 0, 159, 146, 150, 0, 0, 0, 0][..][..];
    assert_matches!(read_string(&mut buf), Ok(StringOrByteBuf::String(s)) => assert_eq!(s, "d"));
    assert_matches!(read_string(&mut buf), Ok(StringOrByteBuf::ByteBuf(b)) => assert_eq!(b, &[0, 159, 146, 150]));
    assert_matches!(read_string(&mut buf), Ok(StringOrByteBuf::String(s)) => assert!(s.is_empty()));
    assert_matches!(read_string(&mut buf), Err(Error::SequenceSize(_)));
}

#[test]
fn test_slice_read_raw() {
    let mut buf = &[1, 0, 0, 0, 100, 1, 0, 0, 0, 1, 0, 0, 0, 0][..][..];
    assert_matches!(read_raw(&mut buf), Ok(s) => assert_eq!(s, Bytes::from_static(&[100])));
    assert_matches!(read_raw(&mut buf), Ok(s) => assert_eq!(s, Bytes::from_static(&[1])));
    assert_matches!(read_raw(&mut buf), Ok(s) => assert_eq!(s, Bytes::from_static(&[])));
    assert_matches!(read_raw(&mut buf), Err(Error::SequenceSize(_)));
}

#[test]
fn test_read_bool() {
    let mut buf = &[0, 1, 2][..];
    assert_matches!(read_bool(&mut buf), Ok(false));
    assert_matches!(read_bool(&mut buf), Ok(true));
    assert_matches!(read_bool(&mut buf), Err(Error::NotABoolValue(2)));
    assert_matches!(read_bool(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_u8() {
    let mut buf = &[0, 1, 2][..];
    assert_matches!(read_u8(&mut buf), Ok(0));
    assert_matches!(read_u8(&mut buf), Ok(1));
    assert_matches!(read_u8(&mut buf), Ok(2));
    assert_matches!(read_u8(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_i8() {
    let mut buf = &[0, 1, 2][..];
    assert_matches!(read_i8(&mut buf), Ok(0));
    assert_matches!(read_i8(&mut buf), Ok(1));
    assert_matches!(read_i8(&mut buf), Ok(2));
    assert_matches!(read_i8(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_u16() {
    let mut buf = &[0, 1, 2, 3, 4][..];
    assert_matches!(read_u16(&mut buf), Ok(256));
    assert_matches!(read_u16(&mut buf), Ok(770));
    assert_matches!(read_u16(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_i16() {
    let mut buf = &[254, 255, 253, 255, 1][..];
    assert_matches!(read_i16(&mut buf), Ok(-2));
    assert_matches!(read_i16(&mut buf), Ok(-3));
    assert_matches!(read_i16(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_u32() {
    let mut buf = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10][..];
    assert_matches!(read_u32(&mut buf), Ok(50462976));
    assert_matches!(read_u32(&mut buf), Ok(117835012));
    assert_matches!(read_u32(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_i32() {
    let mut buf = &[254, 255, 255, 255, 253, 255, 255, 255, 1, 2, 3][..];
    assert_matches!(read_i32(&mut buf), Ok(-2));
    assert_matches!(read_i32(&mut buf), Ok(-3));
    assert_matches!(read_i32(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_u64() {
    let mut buf = &[
        1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0,
    ][..];
    assert_matches!(read_u64(&mut buf), Ok(1));
    assert_matches!(read_u64(&mut buf), Ok(2));
    assert_matches!(read_u64(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_i64() {
    let mut buf = &[
        255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 253, 255,
        255, 255, 255, 255, 255,
    ][..];
    assert_matches!(read_i64(&mut buf), Ok(-1));
    assert_matches!(read_i64(&mut buf), Ok(-2));
    assert_matches!(read_i64(&mut buf), Err(Error::ShortRead));
}

#[test]
#[allow(clippy::float_cmp)]
fn test_read_f32() {
    let mut buf = &[
        0x14, 0xae, 0x29, 0x42, // 42.42
        0xff, 0xff, 0xff, 0x7f, // NaN
        0x00, 0x00, 0x80, 0x7f, // +Infinity
        0x00, 0x00, 0x80, 0xff, // -Infinity
        0x00, 0x00, 0x00, 0x00, // +0
        0x00, 0x00, 0x00, 0x80, // -0
        1, 2, 3,
    ][..];
    assert_matches!(read_f32(&mut buf), Ok(f) if f == 42.42);
    assert_matches!(read_f32(&mut buf), Ok(f) if f.is_nan());
    assert_matches!(read_f32(&mut buf), Ok(f) if f.is_infinite() && f.is_sign_positive());
    assert_matches!(read_f32(&mut buf), Ok(f) if f.is_infinite() && f.is_sign_negative());
    assert_matches!(read_f32(&mut buf), Ok(f) if f == 0.0);
    assert_matches!(read_f32(&mut buf), Ok(f) if f == -0.0);
    assert_matches!(read_f32(&mut buf), Err(Error::ShortRead));
}

#[test]
#[allow(clippy::float_cmp)]
fn test_read_f64() {
    let mut buf = &[
        0xf6, 0x28, 0x5c, 0x8f, 0xc2, 0x35, 0x45, 0x40, // 42.42
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f, // NaN
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x7f, // +Infinity
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0xff, // -Infinity
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // +0
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, // -0
        1, 2, 3, 4, 5, 6, 7,
    ][..];
    assert_matches!(read_f64(&mut buf), Ok(f) if f == 42.42);
    assert_matches!(read_f64(&mut buf), Ok(f) if f.is_nan());
    assert_matches!(read_f64(&mut buf), Ok(f) if f.is_infinite() && f.is_sign_positive());
    assert_matches!(read_f64(&mut buf), Ok(f) if f.is_infinite() && f.is_sign_negative());
    assert_matches!(read_f64(&mut buf), Ok(f) if f == 0.0);
    assert_matches!(read_f64(&mut buf), Ok(f) if f == -0.0);
    assert_matches!(read_f64(&mut buf), Err(Error::ShortRead));
}

#[test]
fn test_read_size() {
    let mut buf = &[0x01, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 1, 2, 3][..];
    assert_matches!(read_size(&mut buf), Ok(1));
    assert_matches!(read_size(&mut buf), Ok(s) if s == u32::MAX as usize);
    assert_matches!(read_size(&mut buf), Err(Error::ShortRead));
}
