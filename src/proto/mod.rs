mod de;
pub mod message;
pub use message::Message;
mod ser;
pub mod utils;
pub mod value;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Custom(String),
}

type Result<T> = std::result::Result<T, Error>;

pub fn to_writer<W, T>(writer: W, value: &T) -> Result<W>
where
    W: std::io::Write,
    T: ?Sized + serde::Serialize,
{
    value.serialize(ser::WriterSerializer::from_writer(writer))
}

pub fn to_bytes_buf<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + serde::Serialize,
{
    let mut buf = Vec::new();
    to_writer(&mut buf, value)?;
    Ok(buf)
}

pub fn from_reader<'r, R, T>(reader: R) -> Result<T>
where
    R: 'r + std::io::Read,
    T: serde::de::Deserialize<'r>,
{
    T::deserialize(de::ReaderDeserializer::from_reader(reader))
}

pub fn from_bytes<'b, T>(bytes: &'b [u8]) -> Result<T>
where
    T: serde::de::Deserialize<'b>,
{
    from_reader(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_message_to_bytes() {
        use message::*;
        let msg = Message {
            id: 329,
            version: 12,
            kind: Kind::Capability,
            flags: Flags::RETURN_TYPE,
            subject: subject::ServiceDirectory {
                action: action::ServiceDirectory::ServiceReady,
            }
            .into(),
            payload: vec![23u8, 43u8, 230u8, 1u8, 95u8],
        };
        let buf = to_bytes_buf(&msg).unwrap();
        assert_eq!(
            buf,
            vec![
                0x42, 0xde, 0xad, 0x42, // cookie
                0x49, 0x01, 0x00, 0x00, // id
                0x05, 0x00, 0x00, 0x00, // size
                0x0c, 0x00, 0x06, 0x02, // version, type, flags
                0x01, 0x00, 0x00, 0x00, // service
                0x01, 0x00, 0x00, 0x00, // object
                0x68, 0x00, 0x00, 0x00, // action
                0x17, 0x2b, 0xe6, 0x01, 0x5f, // payload
            ]
        );
    }

    #[test]
    fn test_message_from_bytes() {
        let input = &[
            0x42, 0xde, 0xad, 0x42, // cookie
            0xb8, 0x9a, 0x00, 0x00, // id
            0x28, 0x00, 0x00, 0x00, // size
            0xaa, 0x00, 0x02, 0x00, // version, type, flags
            0x27, 0x00, 0x00, 0x00, // service
            0x09, 0x00, 0x00, 0x00, // object
            0x68, 0x00, 0x00, 0x00, // action
            // payload
            0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d, 0x65,
            0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35, 0x32, 0x2d,
            0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30, 0x33,
            // garbage at the end, should be ignored
            0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
            0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x00,
        ];
        let msg = from_bytes::<Message>(input).unwrap();
        use message::*;
        assert_eq!(
            msg,
            Message {
                id: 39608,
                version: 170,
                kind: Kind::Reply,
                flags: Flags::empty(),
                subject: subject::BoundObject::from_values_unchecked(
                    subject::service::Id(39).into(),
                    subject::object::Id(9).into(),
                    action::BoundObject::BoundFunction(104.into()),
                )
                .into(),
                payload: vec![
                    0x24, 0x00, 0x00, 0x00, 0x39, 0x32, 0x39, 0x36, 0x33, 0x31, 0x36, 0x34, 0x2d,
                    0x65, 0x30, 0x37, 0x66, 0x2d, 0x34, 0x36, 0x35, 0x30, 0x2d, 0x39, 0x64, 0x35,
                    0x32, 0x2d, 0x39, 0x39, 0x35, 0x37, 0x39, 0x38, 0x61, 0x39, 0x61, 0x65, 0x30,
                    0x33,
                ],
            }
        );
    }
}
