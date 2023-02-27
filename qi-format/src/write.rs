use crate::{
    num_bool::{FALSE_BOOL, TRUE_BOOL},
    Error, Raw, Result, Str,
};

pub fn write_byte<W>(mut writer: W, b: u8) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(&[b])?;
    Ok(())
}

pub fn write_word<W>(mut writer: W, w: &[u8; 2]) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(w)?;
    Ok(())
}

pub fn write_dword<W>(mut writer: W, dw: &[u8; 4]) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(dw)?;
    Ok(())
}

pub fn write_qword<W>(mut writer: W, qw: &[u8; 8]) -> Result<()>
where
    W: std::io::Write,
{
    writer.write_all(qw)?;
    Ok(())
}

pub fn write_bool<W>(writer: W, val: bool) -> Result<()>
where
    W: std::io::Write,
{
    write_byte(writer, if val { TRUE_BOOL } else { FALSE_BOOL })
}

pub fn write_u8<W>(writer: W, val: u8) -> Result<()>
where
    W: std::io::Write,
{
    write_byte(writer, val)
}

pub fn write_i8<W>(writer: W, val: i8) -> Result<()>
where
    W: std::io::Write,
{
    write_byte(writer, val as u8)
}

pub fn write_u16<W>(writer: W, val: u16) -> Result<()>
where
    W: std::io::Write,
{
    write_word(writer, &val.to_le_bytes())
}

pub fn write_i16<W>(writer: W, val: i16) -> Result<()>
where
    W: std::io::Write,
{
    write_word(writer, &val.to_le_bytes())
}

pub fn write_u32<W>(writer: W, val: u32) -> Result<()>
where
    W: std::io::Write,
{
    write_dword(writer, &val.to_le_bytes())
}

pub fn write_i32<W>(writer: W, val: i32) -> Result<()>
where
    W: std::io::Write,
{
    write_dword(writer, &val.to_le_bytes())
}

pub fn write_u64<W>(writer: W, val: u64) -> Result<()>
where
    W: std::io::Write,
{
    write_qword(writer, &val.to_le_bytes())
}

pub fn write_i64<W>(writer: W, val: i64) -> Result<()>
where
    W: std::io::Write,
{
    write_qword(writer, &val.to_le_bytes())
}

pub fn write_f32<W>(writer: W, val: f32) -> Result<()>
where
    W: std::io::Write,
{
    write_dword(writer, &val.to_le_bytes())
}

pub fn write_f64<W>(writer: W, val: f64) -> Result<()>
where
    W: std::io::Write,
{
    write_qword(writer, &val.to_le_bytes())
}

pub fn write_size<W>(writer: W, size: usize) -> Result<()>
where
    W: std::io::Write,
{
    let size = std::convert::TryFrom::try_from(size).map_err(Error::SizeConversionError)?;
    write_u32(writer, size)
}

pub fn write_str<W>(writer: W, str: &Str) -> Result<()>
where
    W: std::io::Write,
{
    write_raw(writer, Raw::new(str.as_bytes()))
}

pub fn write_raw<W>(mut writer: W, raw: &Raw) -> Result<()>
where
    W: std::io::Write,
{
    write_size(writer.by_ref(), raw.len())?;
    writer.write_all(raw)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_byte() {
        let mut buf = Vec::new();
        write_byte(&mut buf, 64).unwrap();
        assert_eq!(buf, [64]);
        write_byte(&mut buf, 65).unwrap();
        assert_eq!(buf, [64, 65]);
    }

    #[test]
    fn test_write_word() {
        let mut buf = Vec::new();
        write_word(&mut buf, &[1, 2]).unwrap();
        assert_eq!(buf, [1, 2]);
        write_word(&mut buf, &[3, 4]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4]);
    }

    #[test]
    fn test_write_dword() {
        let mut buf = Vec::new();
        write_dword(&mut buf, &[1, 2, 3, 4]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4]);
        write_dword(&mut buf, &[5, 6, 7, 8]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_write_qword() {
        let mut buf = Vec::new();
        write_qword(&mut buf, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4, 5, 6, 7, 8]);
        write_qword(&mut buf, &[9, 10, 11, 12, 13, 14, 15, 16]).unwrap();
        assert_eq!(buf, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    }

    #[test]
    fn test_write_bool() {
        let mut buf = Vec::new();
        write_bool(&mut buf, true).unwrap();
        assert_eq!(buf, [1]);
        write_bool(&mut buf, false).unwrap();
        assert_eq!(buf, [1, 0]);
    }

    #[test]
    fn test_write_u8() {
        let mut buf = Vec::new();
        write_u8(&mut buf, 2).unwrap();
        assert_eq!(buf, [2]);
    }

    #[test]
    fn test_write_i8() {
        let mut buf = Vec::new();
        write_i8(&mut buf, -2).unwrap();
        assert_eq!(buf, [254]);
    }

    #[test]
    fn test_write_u16() {
        let mut buf = Vec::new();
        write_u16(&mut buf, 2).unwrap();
        assert_eq!(buf, [2, 0]);
    }

    #[test]
    fn test_write_i16() {
        let mut buf = Vec::new();
        write_i16(&mut buf, -2).unwrap();
        assert_eq!(buf, [254, 255]);
    }

    #[test]
    fn test_write_u32() {
        let mut buf = Vec::new();
        write_u32(&mut buf, 2).unwrap();
        assert_eq!(buf, [2, 0, 0, 0]);
    }

    #[test]
    fn test_write_i32() {
        let mut buf = Vec::new();
        write_i32(&mut buf, -2).unwrap();
        assert_eq!(buf, [254, 255, 255, 255]);
    }

    #[test]
    fn test_write_u64() {
        let mut buf = Vec::new();
        write_u64(&mut buf, 2).unwrap();
        assert_eq!(buf, [2, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_write_i64() {
        let mut buf = Vec::new();
        write_i64(&mut buf, -2).unwrap();
        assert_eq!(buf, [254, 255, 255, 255, 255, 255, 255, 255]);
    }

    #[test]
    fn test_write_f32() {
        let mut buf = Vec::new();
        write_f32(&mut buf, 1.0).unwrap();
        assert_eq!(buf, [0, 0, 128, 63]);

        let mut buf = Vec::new();
        write_f32(&mut buf, 1.0).unwrap();
        assert_eq!(buf, [0, 0, 128, 63]);

        let mut buf = Vec::new();
        write_f32(&mut buf, f32::INFINITY).unwrap();
        assert_eq!(buf, [0x00, 0x00, 0x80, 0x7f]);

        let mut buf = Vec::new();
        write_f32(&mut buf, f32::NEG_INFINITY).unwrap();
        assert_eq!(buf, [0x00, 0x00, 0x80, 0xff]);

        let mut buf = Vec::new();
        write_f32(&mut buf, 0.).unwrap();
        assert_eq!(buf, [0x00, 0x00, 0x00, 0x00]);

        let mut buf = Vec::new();
        write_f32(&mut buf, -0.).unwrap();
        assert_eq!(buf, [0x00, 0x00, 0x00, 0x80]);
    }

    #[test]
    fn test_write_f64() {
        let mut buf = Vec::new();
        write_f64(&mut buf, 1.0).unwrap();
        assert_eq!(buf, [0, 0, 0, 0, 0, 0, 240, 63]);

        let mut buf = Vec::new();
        write_f64(&mut buf, f64::INFINITY).unwrap();
        assert_eq!(buf, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x7f]);

        let mut buf = Vec::new();
        write_f64(&mut buf, f64::NEG_INFINITY).unwrap();
        assert_eq!(buf, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0xff]);

        let mut buf = Vec::new();
        write_f64(&mut buf, 0.).unwrap();
        assert_eq!(buf, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let mut buf = Vec::new();
        write_f64(&mut buf, -0.).unwrap();
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
        write_str(&mut buf, &"abc").unwrap();
        assert_eq!(buf, [3, 0, 0, 0, 97, 98, 99]);
    }

    #[test]
    fn test_write_raw() {
        let mut buf = Vec::new();
        write_raw(&mut buf, &Raw::new(&[1, 11, 111][..])).unwrap();
        assert_eq!(buf, [3, 0, 0, 0, 1, 11, 111]);
    }
}
