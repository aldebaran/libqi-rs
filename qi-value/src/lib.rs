#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

pub mod dynamic;
pub mod map;
mod num_bool;
pub mod object;
mod signature;
mod tuple;
pub mod ty;
mod value;

#[doc(inline)]
pub use crate::{
    dynamic::Dynamic,
    map::Map,
    num_bool::{Float32, Float64, Number},
    object::Object, // TODO: move object out of this crate
    signature::Signature,
    tuple::Tuple,
    ty::Type,
    value::Value,
};

pub use bytes;
pub use bytes::Bytes as Raw;

pub use std::vec::Vec as List;

#[macro_export]
macro_rules! list {
    ($($tt:tt)*) => {
        vec![$($tt)*]
    }
}

#[derive(Debug)]
pub struct DisplayRaw<'a>(pub &'a Raw);

impl<'a> std::fmt::Display for DisplayRaw<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_raw(self.0)
    }
}

#[derive(Debug)]
pub struct DisplayBytes<'a>(pub &'a [u8]);

impl<'a> std::fmt::Display for DisplayBytes<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_raw(self.0)
    }
}

pub trait FormatterExt {
    fn write_option<T>(&mut self, option: &Option<T>) -> std::fmt::Result
    where
        T: std::fmt::Display;

    fn write_raw(&mut self, raw: &[u8]) -> std::fmt::Result;

    fn write_list<T>(&mut self, list: &[T]) -> std::fmt::Result
    where
        T: std::fmt::Display;
}

impl<'a> FormatterExt for std::fmt::Formatter<'a> {
    fn write_option<T>(&mut self, option: &Option<T>) -> std::fmt::Result
    where
        T: std::fmt::Display,
    {
        match option {
            Some(v) => write!(self, "some({v})"),
            None => self.write_str("none"),
        }
    }

    fn write_raw(&mut self, raw: &[u8]) -> std::fmt::Result {
        for byte in raw {
            write!(self, "\\x{byte:x}")?;
        }
        Ok(())
    }

    fn write_list<T>(&mut self, list: &[T]) -> std::fmt::Result
    where
        T: std::fmt::Display,
    {
        let mut add_sep = false;
        for element in list {
            if add_sep {
                self.write_str(", ")?;
            }
            element.fmt(self)?;
            add_sep = true;
        }
        Ok(())
    }
}
