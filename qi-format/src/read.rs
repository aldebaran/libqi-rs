use crate::{Error, Result};
use bytes::Buf;

pub fn read_bool<B: Buf>(bytes: &mut B) -> Result<bool> {
    if !bytes.has_remaining() {
        return Err(Error::ShortRead);
    };
    match bytes.get_u8() {
        crate::FALSE_BOOL => Ok(false),
        crate::TRUE_BOOL => Ok(true),
        byte => Err(Error::NotABoolValue(byte)),
    }
}

pub fn read_u8<B: Buf>(bytes: &mut B) -> Result<u8> {
    if !bytes.has_remaining() {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_u8())
}

pub fn read_i8<B: Buf>(bytes: &mut B) -> Result<i8> {
    if !bytes.has_remaining() {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_i8())
}

pub fn read_u16<B: Buf>(bytes: &mut B) -> Result<u16> {
    if bytes.remaining() < 2 {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_u16_le())
}

pub fn read_i16<B: Buf>(bytes: &mut B) -> Result<i16> {
    if bytes.remaining() < 2 {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_i16_le())
}

pub fn read_u32<B: Buf>(bytes: &mut B) -> Result<u32> {
    if bytes.remaining() < 4 {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_u32_le())
}

pub fn read_i32<B: Buf>(bytes: &mut B) -> Result<i32> {
    if bytes.remaining() < 4 {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_i32_le())
}

pub fn read_u64<B: Buf>(bytes: &mut B) -> Result<u64> {
    if bytes.remaining() < 8 {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_u64_le())
}

pub fn read_i64<B: Buf>(bytes: &mut B) -> Result<i64> {
    if bytes.remaining() < 8 {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_i64_le())
}

pub fn read_f32<B: Buf>(bytes: &mut B) -> Result<f32> {
    if bytes.remaining() < 4 {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_f32_le())
}

pub fn read_f64<B: Buf>(bytes: &mut B) -> Result<f64> {
    if bytes.remaining() < 8 {
        return Err(Error::ShortRead);
    };
    Ok(bytes.get_f64_le())
}

pub fn read_size<B: Buf>(bytes: &mut B) -> Result<usize> {
    let size_as_u32 = read_u32(bytes)?;
    let size = size_as_u32.try_into().map_err(Error::SizeConversionError)?;
    Ok(size)
}

pub fn read_raw<'b>(bytes: &mut &'b [u8]) -> Result<&'b [u8]> {
    let size = read_size(bytes).map_err(|err| Error::SequenceSize(err.into()))?;
    if bytes.remaining() < size {
        return Err(Error::ShortRead);
    }
    let (front, back) = bytes.split_at(size);
    *bytes = back;
    Ok(front)
}

pub fn read_raw_buf<B: Buf>(buf: &mut B) -> Result<Vec<u8>> {
    let size = read_size(buf).map_err(|err| Error::SequenceSize(err.into()))?;
    if buf.remaining() < size {
        return Err(Error::ShortRead);
    }
    let mut raw = vec![0; size];
    buf.copy_to_slice(&mut raw);
    Ok(raw)
}

// equivalence: string -> raw
pub fn read_str<'b>(bytes: &mut &'b [u8]) -> Result<StrOrBytes<'b>> {
    let raw = read_raw(bytes)?;
    Ok(match std::str::from_utf8(raw) {
        Ok(str) => StrOrBytes::Str(str),
        Err(_) => StrOrBytes::Bytes(raw),
    })
}

// equivalence: string -> raw
pub fn read_string<B: Buf>(buf: &mut B) -> Result<StringOrByteBuf> {
    let raw = read_raw_buf(buf)?;
    Ok(match String::from_utf8(raw) {
        Ok(str) => StringOrByteBuf::String(str),
        Err(err) => StringOrByteBuf::ByteBuf(err.into_bytes()),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StrOrBytes<'a> {
    Str(&'a str),
    Bytes(&'a [u8]),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StringOrByteBuf {
    String(String),
    ByteBuf(Vec<u8>),
}
