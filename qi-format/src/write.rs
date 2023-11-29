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

#[cfg(test)]
mod tests {
    use super::*;

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
}
