use qi_format::write::{
    write_bool, write_f32, write_f64, write_i16, write_i32, write_i64, write_i8, write_raw,
    write_size, write_str, write_u16, write_u32, write_u64, write_u8,
};

#[test]
fn test_write_bool() {
    let mut buf = Vec::new();
    write_bool(&mut buf, true);
    assert_eq!(buf, [1]);
    write_bool(&mut buf, false);
    assert_eq!(buf, [1, 0]);
}

#[test]
fn test_write_u8() {
    let mut buf = Vec::new();
    write_u8(&mut buf, 2);
    assert_eq!(buf, [2]);
}

#[test]
fn test_write_i8() {
    let mut buf = Vec::new();
    write_i8(&mut buf, -2);
    assert_eq!(buf, [254]);
}

#[test]
fn test_write_u16() {
    let mut buf = Vec::new();
    write_u16(&mut buf, 2);
    assert_eq!(buf, [2, 0]);
}

#[test]
fn test_write_i16() {
    let mut buf = Vec::new();
    write_i16(&mut buf, -2);
    assert_eq!(buf, [254, 255]);
}

#[test]
fn test_write_u32() {
    let mut buf = Vec::new();
    write_u32(&mut buf, 2);
    assert_eq!(buf, [2, 0, 0, 0]);
}

#[test]
fn test_write_i32() {
    let mut buf = Vec::new();
    write_i32(&mut buf, -2);
    assert_eq!(buf, [254, 255, 255, 255]);
}

#[test]
fn test_write_u64() {
    let mut buf = Vec::new();
    write_u64(&mut buf, 2);
    assert_eq!(buf, [2, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_write_i64() {
    let mut buf = Vec::new();
    write_i64(&mut buf, -2);
    assert_eq!(buf, [254, 255, 255, 255, 255, 255, 255, 255]);
}

#[test]
fn test_write_f32() {
    let mut buf = Vec::new();
    write_f32(&mut buf, 1.0);
    assert_eq!(buf, [0, 0, 128, 63]);

    let mut buf = Vec::new();
    write_f32(&mut buf, 1.0);
    assert_eq!(buf, [0, 0, 128, 63]);

    let mut buf = Vec::new();
    write_f32(&mut buf, f32::INFINITY);
    assert_eq!(buf, [0x00, 0x00, 0x80, 0x7f]);

    let mut buf = Vec::new();
    write_f32(&mut buf, f32::NEG_INFINITY);
    assert_eq!(buf, [0x00, 0x00, 0x80, 0xff]);

    let mut buf = Vec::new();
    write_f32(&mut buf, 0.);
    assert_eq!(buf, [0x00, 0x00, 0x00, 0x00]);

    let mut buf = Vec::new();
    write_f32(&mut buf, -0.);
    assert_eq!(buf, [0x00, 0x00, 0x00, 0x80]);
}

#[test]
fn test_write_f64() {
    let mut buf = Vec::new();
    write_f64(&mut buf, 1.0);
    assert_eq!(buf, [0, 0, 0, 0, 0, 0, 240, 63]);

    let mut buf = Vec::new();
    write_f64(&mut buf, f64::INFINITY);
    assert_eq!(buf, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x7f]);

    let mut buf = Vec::new();
    write_f64(&mut buf, f64::NEG_INFINITY);
    assert_eq!(buf, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0xff]);

    let mut buf = Vec::new();
    write_f64(&mut buf, 0.);
    assert_eq!(buf, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    let mut buf = Vec::new();
    write_f64(&mut buf, -0.);
    assert_eq!(buf, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80]);
}

#[test]
fn test_write_size() {
    let mut buf = Vec::new();
    write_size(&mut buf, 2).unwrap();
    assert_eq!(buf, [2, 0, 0, 0]);
}

#[test]
fn test_write_string() {
    let mut buf = Vec::new();
    write_str(&mut buf, "abc").unwrap();
    assert_eq!(buf, [3, 0, 0, 0, 97, 98, 99]);
}

#[test]
fn test_write_raw() {
    let mut buf = Vec::new();
    write_raw(&mut buf, &[1, 11, 111][..]).unwrap();
    assert_eq!(buf, [3, 0, 0, 0, 1, 11, 111]);
}
