use assert_matches::assert_matches;
use bytes::{Bytes, BytesMut};
use qi_messaging::{
    binary_codec::{DecodeError, Decoder, Encoder},
    message::{Action, Address, Id, Object, Service, Version},
    CapabilitiesMap, Message,
};
use qi_value::{Dynamic, IntoValue};
use serde_json::json;

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
    let mut decoder = Decoder::<JsonBody>::new();
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
    let mut decoder = Decoder::<JsonBody>::new();
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
    let mut decoder = Decoder::<JsonBody>::new();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Err(DecodeError::UnsupportedVersion(Version(0x3412))));
}

#[test]
fn decoder_not_enough_data_for_header() {
    let data = [0x42, 0xde, 0xad];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::<JsonBody>::new();
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
    let mut decoder = Decoder::<JsonBody>::new();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Ok(None));
}

#[test]
fn decoder_garbage_magic_cookie() {
    let data = [1; 64];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::<JsonBody>::new();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(res, Err(DecodeError::InvalidMagicCookieValue(0x01010101)));
}

#[test]
fn decoder_success() {
    let data = [
        0x42, 0xde, 0xad, 0x42, // cookie
        1, 0, 0, 0, // id
        4, 0, 0, 0, // size
        0, 0, 5, 2, // version, type, flags
        1, 0, 0, 0, // service
        1, 0, 0, 0, // object
        1, 0, 0, 0, // action
        b'"', b'h', b'i', b'"', // body
    ];
    let mut buf = BytesMut::from_iter(data);
    let mut decoder = Decoder::<JsonBody>::new();
    let res = tokio_util::codec::Decoder::decode(&mut decoder, &mut buf);
    assert_matches!(
        res,
        Ok(Some(Message::Event {
            id: Id(1),
            address: Address(Service(1), Object(1), Action(1)),
            value: JsonBody(serde_json::Value::String(s))
        })) if s == "hi"
    );
}

#[test]
fn encoder_success() {
    let message = Message::Call {
        id: Id(1),
        address: Address::DEFAULT,
        value: JsonBody(json! {[1, 2, 3]}),
    };
    let mut encoder_buf = BytesMut::new();
    let res = tokio_util::codec::Encoder::encode(&mut Encoder, message, &mut encoder_buf);
    assert_matches!(res, Ok(()));
}

#[test]
fn message_encode() {
    let msg = Message::<JsonBody>::Capabilities {
        id: Id(329),
        address: Address(Service(1), Object(1), Action(104)),
        capabilities: CapabilitiesMap::from_iter([(
            "hello".to_owned(),
            Dynamic("world".into_value()),
        )]),
    };
    let mut buf = BytesMut::new();
    let mut encoder = Encoder;
    let res = tokio_util::codec::Encoder::encode(&mut encoder, msg, &mut buf);

    assert_matches!(res, Ok(()));
    assert_eq!(
        buf.as_ref(),
        [
            0x42, 0xde, 0xad, 0x42, // cookie
            0x49, 0x01, 0x00, 0x00, // id
            0x2b, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x06, 0x00, // version, type, flags
            0x01, 0x00, 0x00, 0x00, // service
            0x01, 0x00, 0x00, 0x00, // object
            0x68, 0x00, 0x00, 0x00, // action
            b'{', b'"', b'h', b'e', b'l', b'l', b'o', b'"', b':', b'{', b'"', b's', b'i', b'g',
            b'n', b'a', b't', b'u', b'r', b'e', b'"', b':', b'"', b's', b'"', b',', b'"', b'v',
            b'a', b'l', b'u', b'e', b'"', b':', b'"', b'w', b'o', b'r', b'l', b'd', b'"', b'}',
            b'}', // body
        ]
        .as_slice()
    );
}

#[derive(Debug, PartialEq, Eq)]
struct JsonBody(serde_json::Value);

impl qi_messaging::BodyBuf for JsonBody {
    type Error = serde_json::Error;
    type Data = Bytes;

    fn from_bytes(bytes: Bytes) -> Result<Self, Self::Error> {
        serde_json::from_slice(&bytes).map(Self)
    }

    fn into_data(self) -> Result<Self::Data, Self::Error> {
        serde_json::to_vec(&self.0).map(Into::into)
    }

    fn serialize<T>(value: &T) -> Result<Self, Self::Error>
    where
        T: serde::Serialize,
    {
        serde_json::to_value(value).map(Self)
    }

    fn deserialize<'de, T>(&'de self) -> Result<T, Self::Error>
    where
        T: serde::de::Deserialize<'de>,
    {
        T::deserialize(&self.0)
    }
}
