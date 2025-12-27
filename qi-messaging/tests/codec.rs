use assert_matches::assert_matches;
use bytes::{Bytes, BytesMut};
use qi_messaging::{
    codec::{DecodeError, Decoder, Encoder},
    message::{Action, Address, Id, Object, Service, Version},
    Message,
};
use qi_value::{IntoValue, KeyDynValueMap};

#[test]
fn decoder_invalid_magic_cookie_value() {
    let data = [
        0x42, 0xdf, 0xad, 0x42, 0x84, 0x1c, 0x0f, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
        0x00, 0x2f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x00, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x73, 0x1a, 0x00, 0x00, 0x00, 0x54, 0x68, 0x65, 0x20, 0x72, 0x6f, 0x62, 0x6f,
        0x74, 0x20, 0x69, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x69,
        0x7a, 0x65, 0x64,
    ];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::default();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Err(DecodeError::InvalidMagicCookieValue(0x42dfad42)));
}

#[test]
fn decoder_invalid_type_value() {
    let data = [
        0x42, 0xde, 0xad, 0x42, // cookie,
        0x84, 0x1c, 0x0f, 0x00, // id
        0x23, 0x00, 0x00, 0x00, // size
        0x00, 0x00, 12, 0x00, // version, type, flags
        0x2f, 0x00, 0x00, 0x00, // service
        0x01, 0x00, 0x00, 0x00, // action
        0xb2, 0x00, 0x00, 0x00, // action
    ];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::default();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Err(DecodeError::InvalidTypeValue(12)));
}

#[test]
fn decoder_unsupported_version() {
    let data = [
        0x42, 0xde, 0xad, 0x42, // cookie,
        0x84, 0x1c, 0x0f, 0x00, // id
        0x23, 0x00, 0x00, 0x00, // size
        0x12, 0x34, 0x03, 0x00, // version, type, flags
        0x2f, 0x00, 0x00, 0x00, // service
        0x01, 0x00, 0x00, 0x00, // object
        0xb2, 0x00, 0x00, 0x00, // address
    ];

    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::default();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Err(DecodeError::UnsupportedVersion(Version(0x3412))));
}

#[test]
fn decoder_not_enough_data_for_header() {
    let data = [0x42, 0xde, 0xad];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::default();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Ok(None));
}

#[test]
fn decoder_not_enough_data_for_body() {
    let data = [
        0x42, 0xde, 0xad, 0x42, // cookie
        1, 0, 0, 0, // id
        5, 0, 0, 0, // size
        0, 0, 5, 2, // version, type, flags
        1, 0, 0, 0, // service
        1, 0, 0, 0, // object
        1, 0, 0, 0, // action
        1, 2, 3, // body
    ];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::default();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Ok(None));
}

#[test]
fn decoder_garbage_magic_cookie() {
    let data = [1; 64];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::default();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Err(DecodeError::InvalidMagicCookieValue(0x01010101)));
}

#[test]
fn decoder_success() {
    let data = [
        0x42, 0xde, 0xad, 0x42, // cookie
        1, 0, 0, 0, // id
        6, 0, 0, 0, // size
        0, 0, 5, 2, // version, type, flags
        1, 0, 0, 0, // service
        1, 0, 0, 0, // object
        1, 0, 0, 0, // action
        // body
        2, 0, 0, 0, b'h', b'i',
    ];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::default();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(
        res,
        Ok(Some(Message::Event {
            id: Id(1),
            address: Address(Service(1), Object(1), Action(1)),
            payload
        })) => {
            assert_eq!(payload, [2, 0, 0, 0, b'h', b'i'].as_slice());
        }
    );
}

#[test]
fn encoder_success() {
    let message = Message::Call {
        id: Id(1),
        address: Address::default(),
        payload: Bytes::from_static(&[1, 2, 3]),
    };
    let mut encoder_buf = BytesMut::new();
    let res = tokio_util::codec::Encoder::encode(&mut Encoder, message, &mut encoder_buf);
    assert_matches!(res, Ok(()));
}

#[test]
fn message_encode() {
    let msg = Message::Capabilities {
        id: Id(329),
        address: Address(Service(1), Object(1), Action(104)),
        capabilities: KeyDynValueMap::from_iter([("hello".to_owned(), "world".into_value())]),
    };
    let mut buf = BytesMut::new();
    let mut encoder = Encoder;
    let res = tokio_util::codec::Encoder::encode(&mut encoder, msg, &mut buf);

    assert_matches!(res, Ok(()));
    assert_eq!(
        &buf[..28],
        [
            0x42, 0xde, 0xad, 0x42, // cookie
            0x49, 0x01, 0x00, 0x00, // id
            27, 0, 0, 0, // size
            0x00, 0x00, 0x06, 0x00, // version, type, flags
            0x01, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0x68, 0x00, 0x00, 0x00, // action
        ]
        .as_slice()
    );
}
