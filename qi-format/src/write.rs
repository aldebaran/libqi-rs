use crate::{Error, FALSE_BOOL, TRUE_BOOL};
use bytes::BufMut;

pub fn write_bool<B>(buf: &mut B, val: bool)
where
    B: BufMut,
{
    buf.put_u8(if val { TRUE_BOOL } else { FALSE_BOOL });
}

pub fn write_u8<B>(buf: &mut B, val: u8)
where
    B: BufMut,
{
    buf.put_u8(val)
}

pub fn write_i8<B>(buf: &mut B, val: i8)
where
    B: BufMut,
{
    buf.put_i8(val)
}

pub fn write_u16<B>(buf: &mut B, val: u16)
where
    B: BufMut,
{
    buf.put_u16_le(val)
}

pub fn write_i16<B>(buf: &mut B, val: i16)
where
    B: BufMut,
{
    buf.put_i16_le(val)
}

pub fn write_u32<B>(buf: &mut B, val: u32)
where
    B: BufMut,
{
    buf.put_u32_le(val)
}

pub fn write_i32<B>(buf: &mut B, val: i32)
where
    B: BufMut,
{
    buf.put_i32_le(val)
}

pub fn write_u64<B>(buf: &mut B, val: u64)
where
    B: BufMut,
{
    buf.put_u64_le(val)
}

pub fn write_i64<B>(buf: &mut B, val: i64)
where
    B: BufMut,
{
    buf.put_i64_le(val)
}

pub fn write_f32<B>(buf: &mut B, val: f32)
where
    B: BufMut,
{
    buf.put_f32_le(val)
}

pub fn write_f64<B>(buf: &mut B, val: f64)
where
    B: BufMut,
{
    buf.put_f64_le(val)
}

pub fn write_size<B>(buf: &mut B, size: usize) -> crate::Result<()>
where
    B: BufMut,
{
    let size = std::convert::TryFrom::try_from(size).map_err(Error::SizeConversionError)?;
    buf.put_u32_le(size);
    Ok(())
}

pub fn write_str<B>(buf: &mut B, str: &str) -> crate::Result<()>
where
    B: BufMut,
{
    write_raw(buf, str.as_bytes())
}

pub fn write_raw<B>(buf: &mut B, raw: &[u8]) -> crate::Result<()>
where
    B: BufMut,
{
    write_size(buf, raw.len())?;
    buf.put(raw);
    Ok(())
}
