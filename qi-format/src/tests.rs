use super::*;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct S0 {
    u: (),
    t: (i8, u8, i16, u16, i32, u32, i64, u64, f32, f64),
    #[serde(with = "serde_bytes")]
    r: Vec<u8>,
    o: std::option::Option<bool>,
    s: S1,
    l: Vec<std::string::String>,
    m: BTreeMap<i32, std::string::String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct S1(std::string::String, std::string::String);

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Serializable(S0);

#[test]
fn test_to_from_bytes_serializable() {
    let sample_in = Serializable(S0 {
        u: (),
        t: (-8, 8, -16, 16, -32, 32, -64, 64, 32.32, 64.64),
        r: vec![51, 52, 53, 54],
        o: Some(false),
        s: S1("bananas".to_string(), "oranges".to_string()),
        l: vec!["cookies".to_string(), "muffins".to_string()],
        m: {
            let mut m = BTreeMap::new();
            m.insert(1, "hello".to_string());
            m.insert(2, "world".to_string());
            m
        },
    });
    let expected_bytes = [
        0xf8, 0x08, 0xf0, 0xff, 0x10, 0x00, 0xe0, 0xff, 0xff, 0xff, 0x20, 0x00, 0x00, 0x00, 0xc0,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xae, 0x47, 0x01, 0x42, 0x29, 0x5c, 0x8f, 0xc2, 0xf5, 0x28, 0x50, 0x40, // t
        4, 0, 0, 0, 51, 52, 53, 54, // r
        1, 0, // o
        7, 0, 0, 0, b'b', b'a', b'n', b'a', b'n', b'a', b's', 7, 0, 0, 0, b'o', b'r', b'a', b'n',
        b'g', b'e', b's', // s
        2, 0, 0, 0, 7, 0, 0, 0, b'c', b'o', b'o', b'k', b'i', b'e', b's', 7, 0, 0, 0, b'm', b'u',
        b'f', b'f', b'i', b'n', b's', // l
        2, 0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o', 2, 0, 0, 0, 5, 0, 0, 0,
        b'w', b'o', b'r', b'l', b'd', // m
    ];
    let actual_bytes = to_bytes(&sample_in).unwrap();
    assert_eq!(actual_bytes, expected_bytes);
    let sample_out: Serializable = from_bytes(&actual_bytes).unwrap();
    assert_eq!(sample_in, sample_out);
}
