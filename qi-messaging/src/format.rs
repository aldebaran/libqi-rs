use bytes::{BufMut, Bytes, BytesMut};
pub use qi_format::{from_bytes, Error};

pub fn to_bytes<T>(value: &T) -> Result<Bytes, Error>
where
    T: serde::Serialize,
{
    let mut writer = BytesMut::new().writer();
    qi_format::to_writer(&mut writer, value)?;
    Ok(writer.into_inner().freeze())
}
