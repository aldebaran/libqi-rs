mod dynamic;
mod map;
mod num_bool;
mod object;
mod signature;
mod tuple;
mod ty;
mod value;

#[doc(inline)]
pub use crate::{
    dynamic::Dynamic,
    map::Map,
    num_bool::{Float32, Float64, Number},
    object::{MetaMethod, MetaObject, MetaProperty, MetaSignal, Object},
    signature::Signature,
    tuple::Tuple,
    ty::Type,
    value::Value,
};

pub use bytes;
pub use bytes::Bytes as Raw;

pub use std::vec::Vec as List;

impl ty::StaticGetType for String {
    fn get_type() -> Type {
        Type::String
    }
}

impl ty::StaticGetType for Raw {
    fn get_type() -> Type {
        Type::Raw
    }
}

impl<T> ty::StaticGetType for Option<T>
where
    T: ty::StaticGetType,
{
    fn get_type() -> Type {
        ty::option_of(T::get_type())
    }
}

impl ty::DynamicGetType for Option<Value> {
    fn get_type(&self) -> Type {
        match self {
            Some(value) => ty::option_of(value.get_type()),
            None => Type::Option(None),
        }
    }
}

impl ty::DynamicGetType for Option<Dynamic> {
    fn get_type(&self) -> Type {
        Type::Option(None)
    }
}

impl<T> ty::StaticGetType for List<T>
where
    T: ty::StaticGetType,
{
    fn get_type() -> Type {
        ty::list_of(T::get_type())
    }
}

impl ty::DynamicGetType for List<Value> {
    fn get_type(&self) -> Type {
        let t = self
            .iter()
            .map(|value| Some(value.get_type()))
            .reduce(ty::common_type)
            .flatten();
        Type::List(t.map(Box::new))
    }
}

impl ty::DynamicGetType for List<Dynamic> {
    fn get_type(&self) -> Type {
        Type::List(None)
    }
}

#[macro_export]
macro_rules! list {
    ($($tt:tt)*) => {
        vec![$($tt)*]
    }
}

trait FormatterExt {
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
