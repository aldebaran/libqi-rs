use crate::{
    num_bool::{FALSE_BOOL, TRUE_BOOL},
    Error, Raw, Result, String,
};
use derive_new::new;

mod private {
    pub trait Sealed {}
}

pub trait Read: private::Sealed {
    type String;
    type Raw;

    fn read_byte(&mut self) -> Result<u8>;
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]>;
    fn read_string(&mut self) -> Result<Self::String>;
    fn read_raw(&mut self) -> Result<Self::Raw>;

    fn read_word(&mut self) -> Result<[u8; 2]> {
        self.read_bytes()
    }

    fn read_dword(&mut self) -> Result<[u8; 4]> {
        self.read_bytes()
    }

    fn read_qword(&mut self) -> Result<[u8; 8]> {
        self.read_bytes()
    }

    fn read_bool(&mut self) -> Result<bool> {
        let byte = self.read_byte()?;
        match byte {
            FALSE_BOOL => Ok(false),
            TRUE_BOOL => Ok(true),
            _ => Err(Error::NotABoolValue(byte)),
        }
    }

    fn read_u8(&mut self) -> Result<u8> {
        let byte = self.read_byte()?;
        Ok(u8::from_le(byte))
    }

    fn read_i8(&mut self) -> Result<i8> {
        let byte = self.read_byte()?;
        Ok(i8::from_le(byte as i8))
    }

    fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.read_bytes()?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_i16(&mut self) -> Result<i16> {
        let bytes = self.read_bytes()?;
        Ok(i16::from_le_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_bytes()?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_i32(&mut self) -> Result<i32> {
        let bytes = self.read_bytes()?;
        Ok(i32::from_le_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.read_bytes()?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_i64(&mut self) -> Result<i64> {
        let bytes = self.read_bytes()?;
        Ok(i64::from_le_bytes(bytes))
    }

    fn read_f32(&mut self) -> Result<f32> {
        let bytes = self.read_bytes()?;
        Ok(f32::from_le_bytes(bytes))
    }

    fn read_f64(&mut self) -> Result<f64> {
        let bytes = self.read_bytes()?;
        Ok(f64::from_le_bytes(bytes))
    }

    fn read_size(&mut self) -> Result<usize> {
        let size_bytes = self.read_bytes()?;
        let size = u32::from_le_bytes(size_bytes)
            .try_into()
            .map_err(Error::SizeConversionError)?;
        Ok(size)
    }

    fn as_ref(&mut self) -> &mut Self {
        self
    }
}

impl<R> private::Sealed for &mut R where R: Read {}

impl<R> Read for &mut R
where
    R: Read,
{
    type String = <R as Read>::String;
    type Raw = <R as Read>::Raw;

    fn read_byte(&mut self) -> Result<u8> {
        (*self).read_byte()
    }

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        (*self).read_bytes()
    }

    fn read_string(&mut self) -> Result<Self::String> {
        (*self).read_string()
    }

    fn read_raw(&mut self) -> Result<Self::Raw> {
        (*self).read_raw()
    }
}

#[derive(new, Debug)]
pub struct IoRead<R> {
    reader: R,
}

impl<R> private::Sealed for IoRead<R> where R: std::io::Read {}

impl<R> Read for IoRead<R>
where
    R: std::io::Read,
{
    type String = String<'static>;
    type Raw = Raw<'static>;

    fn read_byte(&mut self) -> Result<u8> {
        let mut byte = 0;
        self.reader.read_exact(std::slice::from_mut(&mut byte))?;
        Ok(byte)
    }

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buf = [0; N];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    // equivalence: string -> raw
    fn read_string(&mut self) -> Result<Self::String> {
        let raw = self.read_raw()?;
        Ok(String::from(raw))
    }

    fn read_raw(&mut self) -> Result<Self::Raw> {
        let size = self.read_size()?;
        let mut buf = vec![0; size];
        self.reader.read_exact(&mut buf)?;
        Ok(Raw::from(buf))
    }
}

#[derive(new, Debug)]
pub struct SliceRead<'b> {
    data: &'b [u8],
}

impl<'b> private::Sealed for SliceRead<'b> {}

impl<'b> Read for SliceRead<'b> {
    type String = String<'b>;
    type Raw = Raw<'b>;

    fn read_byte(&mut self) -> Result<u8> {
        let (&byte, tail) = self.data.split_first().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "no data",
            ))
        })?;
        self.data = tail;
        Ok(byte)
    }

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        IoRead::new(&mut self.data).read_bytes()
    }

    fn read_string(&mut self) -> Result<Self::String> {
        self.read_raw().map(String::from)
    }

    fn read_raw(&mut self) -> Result<Self::Raw> {
        let size = self.read_size()?;
        if size > self.data.len() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "data length inconsistent with raw/string size",
            )));
        }
        let (head, tail) = self.data.split_at(size);
        self.data = tail;
        Ok(Raw::from_bytes(head))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn test_io_read_byte() {
        let mut read = IoRead::new(&[1, 2][..]);
        assert_matches!(read.read_byte(), Ok(1));
        assert_matches!(read.read_byte(), Ok(2));
        assert_matches!(read.read_byte(), Err(Error::Io(_)));
    }

    #[test]
    fn test_io_read_bytes() {
        let mut read = IoRead::new(&[1, 2, 3, 4, 5][..]);
        assert_matches!(read.read_bytes::<1>(), Ok([1]));
        assert_matches!(read.read_bytes::<2>(), Ok([2, 3]));
        assert_matches!(read.read_bytes::<3>(), Err(Error::Io(_)));
        assert_matches!(read.read_bytes::<2>(), Ok([4, 5]));
    }

    #[test]
    fn test_io_read_string() {
        let mut read = IoRead::new(&[3, 0, 0, 0, 97, 98, 99, 2, 0, 0, 0, 1, 2][..]);
        assert_matches!(read.read_string(), Ok(s) => s == String::from("abc"));
        assert_matches!(read.read_string(), Ok(s) => s == String::from_bytes(&[1, 2]));
        assert_matches!(read.read_string(), Err(Error::Io(_)));
    }

    #[test]
    fn test_io_read_raw() {
        let mut read = IoRead::new(&[3, 0, 0, 0, 97, 98, 99, 2, 0, 0, 0, 1, 2][..]);
        assert_matches!(read.read_raw(), Ok(s) => s == Raw::from_bytes(&[97, 98, 99]));
        assert_matches!(read.read_raw(), Ok(s) => s == Raw::from_bytes(&[1, 2]));
        assert_matches!(read.read_raw(), Err(Error::Io(_)));
    }

    #[test]
    fn test_slice_read_byte() {
        let mut read = SliceRead::new(&[1, 2]);
        assert_matches!(read.read_byte(), Ok(1));
        assert_matches!(read.read_byte(), Ok(2));
        assert_matches!(read.read_byte(), Err(Error::Io(_)));
    }

    #[test]
    fn test_slice_read_bytes() {
        let mut read = SliceRead::new(&[1, 2, 3, 4, 5]);
        assert_matches!(read.read_bytes::<1>(), Ok([1]));
        assert_matches!(read.read_bytes::<2>(), Ok([2, 3]));
        assert_matches!(read.read_bytes::<3>(), Err(Error::Io(_)));
        assert_matches!(read.read_bytes::<2>(), Ok([4, 5]));
    }

    #[test]
    fn test_slice_read_string() {
        let mut read = SliceRead::new(&[1, 0, 0, 0, 100, 1, 0, 0, 0, 1, 0, 0, 0, 0]);
        assert_matches!(read.read_string(), Ok(s) => s == String::from("d"));
        assert_matches!(read.read_string(), Ok(s) => s == String::from(&[1][..]));
        assert_matches!(read.read_string(), Ok(s) => s == String::new());
        assert_matches!(read.read_string(), Err(Error::Io(_)));
    }

    #[test]
    fn test_slice_read_raw() {
        let mut read = SliceRead::new(&[1, 0, 0, 0, 100, 1, 0, 0, 0, 1, 0, 0, 0, 0]);
        assert_matches!(read.read_raw(), Ok(s) => s == Raw::from_bytes(&[100]));
        assert_matches!(read.read_raw(), Ok(s) => s == Raw::from_bytes(&[1]));
        assert_matches!(read.read_raw(), Ok(s) => s == Raw::from_bytes(&[]));
        assert_matches!(read.read_raw(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_word() {
        let mut read = SliceRead::new(&[1, 2, 3, 4, 5]);
        assert_matches!(read.read_word(), Ok([1, 2]));
        assert_matches!(read.read_word(), Ok([3, 4]));
        assert_matches!(read.read_word(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_dword() {
        let mut read = SliceRead::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_matches!(read.read_dword(), Ok([1, 2, 3, 4]));
        assert_matches!(read.read_dword(), Ok([5, 6, 7, 8]));
        assert_matches!(read.read_dword(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_qword() {
        let mut read = SliceRead::new(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23, 23,
        ]);
        assert_matches!(read.read_qword(), Ok([1, 2, 3, 4, 5, 6, 7, 8]));
        assert_matches!(read.read_qword(), Ok([9, 10, 11, 12, 13, 14, 15, 16]));
        assert_matches!(read.read_qword(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_bool() {
        let mut read = SliceRead::new(&[0, 1, 2]);
        assert_matches!(read.read_bool(), Ok(false));
        assert_matches!(read.read_bool(), Ok(true));
        assert_matches!(read.read_bool(), Err(Error::NotABoolValue(2)));
        assert_matches!(read.read_bool(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_u8() {
        let mut read = SliceRead::new(&[0, 1, 2]);
        assert_matches!(read.read_u8(), Ok(0));
        assert_matches!(read.read_u8(), Ok(1));
        assert_matches!(read.read_u8(), Ok(2));
        assert_matches!(read.read_u8(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_i8() {
        let mut read = SliceRead::new(&[0, 1, 2]);
        assert_matches!(read.read_i8(), Ok(0));
        assert_matches!(read.read_i8(), Ok(1));
        assert_matches!(read.read_i8(), Ok(2));
        assert_matches!(read.read_i8(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_u16() {
        let mut read = SliceRead::new(&[0, 1, 2, 3, 4]);
        assert_matches!(read.read_u16(), Ok(256));
        assert_matches!(read.read_u16(), Ok(770));
        assert_matches!(read.read_u16(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_i16() {
        let mut read = SliceRead::new(&[254, 255, 253, 255, 1]);
        assert_matches!(read.read_i16(), Ok(-2));
        assert_matches!(read.read_i16(), Ok(-3));
        assert_matches!(read.read_i16(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_u32() {
        let mut read = SliceRead::new(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_matches!(read.read_u32(), Ok(50462976));
        assert_matches!(read.read_u32(), Ok(117835012));
        assert_matches!(read.read_u32(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_i32() {
        let mut read = SliceRead::new(&[254, 255, 255, 255, 253, 255, 255, 255, 1, 2, 3]);
        assert_matches!(read.read_i32(), Ok(-2));
        assert_matches!(read.read_i32(), Ok(-3));
        assert_matches!(read.read_i32(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_u64() {
        let mut read = SliceRead::new(&[
            1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0,
        ]);
        assert_matches!(read.read_u64(), Ok(1));
        assert_matches!(read.read_u64(), Ok(2));
        assert_matches!(read.read_u64(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_i64() {
        let mut read = SliceRead::new(&[
            255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 253,
            255, 255, 255, 255, 255, 255,
        ]);
        assert_matches!(read.read_i64(), Ok(-1));
        assert_matches!(read.read_i64(), Ok(-2));
        assert_matches!(read.read_i64(), Err(Error::Io(_)));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_read_f32() {
        let mut read = SliceRead::new(&[
            0x14, 0xae, 0x29, 0x42, // 42.42
            0xff, 0xff, 0xff, 0x7f, // NaN
            0x00, 0x00, 0x80, 0x7f, // +Infinity
            0x00, 0x00, 0x80, 0xff, // -Infinity
            0x00, 0x00, 0x00, 0x00, // +0
            0x00, 0x00, 0x00, 0x80, // -0
            1, 2, 3,
        ]);
        assert_matches!(read.read_f32(), Ok(f) if f == 42.42);
        assert_matches!(read.read_f32(), Ok(f) if f.is_nan());
        assert_matches!(read.read_f32(), Ok(f) if f.is_infinite() && f.is_sign_positive());
        assert_matches!(read.read_f32(), Ok(f) if f.is_infinite() && f.is_sign_negative());
        assert_matches!(read.read_f32(), Ok(f) if f == 0.0);
        assert_matches!(read.read_f32(), Ok(f) if f == -0.0);
        assert_matches!(read.read_f32(), Err(Error::Io(_)));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_read_f64() {
        let mut read = SliceRead::new(&[
            0xf6, 0x28, 0x5c, 0x8f, 0xc2, 0x35, 0x45, 0x40, // 42.42
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f, // NaN
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x7f, // +Infinity
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0xff, // -Infinity
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // +0
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, // -0
            1, 2, 3, 4, 5, 6, 7,
        ]);
        assert_matches!(read.read_f64(), Ok(f) if f == 42.42);
        assert_matches!(read.read_f64(), Ok(f) if f.is_nan());
        assert_matches!(read.read_f64(), Ok(f) if f.is_infinite() && f.is_sign_positive());
        assert_matches!(read.read_f64(), Ok(f) if f.is_infinite() && f.is_sign_negative());
        assert_matches!(read.read_f64(), Ok(f) if f == 0.0);
        assert_matches!(read.read_f64(), Ok(f) if f == -0.0);
        assert_matches!(read.read_f64(), Err(Error::Io(_)));
    }

    #[test]
    fn test_read_size() {
        let mut read = SliceRead::new(&[0x01, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 1, 2, 3]);
        assert_matches!(read.read_size(), Ok(1));
        assert_matches!(read.read_size(), Ok(s) if s == u32::MAX as usize);
        assert_matches!(read.read_size(), Err(Error::Io(_)));
    }
}
