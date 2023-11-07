use crate::{FromValue, FromValueError, IntoValue, Reflect, RuntimeReflect, ToValue, Type, Value};
use std::borrow::Cow;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AsRaw<T>(pub T);

impl<T> Reflect for AsRaw<T> {
    fn ty() -> Option<Type> {
        Some(Type::Raw)
    }
}

impl<T> RuntimeReflect for AsRaw<T> {
    fn ty(&self) -> Type {
        Type::Raw
    }
}

impl<T> From<T> for AsRaw<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> ToValue for AsRaw<T>
where
    T: AsRef<[u8]>,
{
    fn to_value(&self) -> Value<'_> {
        Value::Raw(self.0.as_ref().into())
    }
}

impl<'a> IntoValue<'a> for AsRaw<Vec<u8>> {
    fn into_value(self) -> Value<'a> {
        Value::Raw(self.0.into())
    }
}

impl<'long: 'short, 'short, T> IntoValue<'short> for AsRaw<&'long T>
where
    T: ?Sized + AsRef<[u8]>,
{
    fn into_value(self) -> Value<'short> {
        Value::Raw(self.0.as_ref().into())
    }
}

impl<'long: 'short, 'short> FromValue<'long> for AsRaw<&'short [u8]> {
    fn from_value(value: Value<'long>) -> Result<Self, FromValueError> {
        match value {
            Value::Raw(Cow::Borrowed(r)) => Ok(AsRaw(r)),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a slice of bytes as a raw value".to_owned(),
                actual: value.to_string(),
            }),
        }
    }
}

impl<'a> FromValue<'a> for AsRaw<Vec<u8>> {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        match value {
            Value::Raw(r) => Ok(AsRaw(r.into())),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a byte buffer as a raw value".to_owned(),
                actual: value.to_string(),
            }),
        }
    }
}

impl<'a> FromValue<'a> for AsRaw<Box<[u8]>> {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        match value {
            Value::Raw(r) => Ok(AsRaw(r.into())),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a byte buffer as a raw value".to_owned(),
                actual: value.to_string(),
            }),
        }
    }
}

impl<'long: 'short, 'short> FromValue<'long> for AsRaw<Cow<'short, [u8]>> {
    fn from_value(value: Value<'long>) -> Result<Self, FromValueError> {
        match value {
            Value::Raw(r) => Ok(AsRaw(r)),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a copy-on-write raw value".to_owned(),
                actual: value.to_string(),
            }),
        }
    }
}
