use super::{FromValue, FromValueError, IntoValue, ToValue, Value};
use crate::{ty, Reflect, RuntimeReflect, Type};
use bytes::Bytes;
use ordered_float::OrderedFloat;
use seq_macro::seq;
use std::{borrow::Cow, hash::Hash, rc::Rc, sync::Arc};

macro_rules! impl_reflect {
    (impl $( < $($lt:lifetime,)* $($params:ident),* $(,)*> )? for $t:ty => $vt:ident $(where $($bounds:tt)*)?) => {
        impl $(<$($lt),* $($params),*>)? Reflect for $t $(where $($bounds)*)? {
            fn ty() -> Option<Type> {
                Some(Type::$vt)
            }
        }

        impl $(<$($lt),* $($params),*>)? RuntimeReflect for $t $(where $($bounds)*)? {
            fn ty(&self) -> Type {
                Type::$vt
            }
        }
    };
}

impl_reflect!(impl for () => Unit);

impl ToValue for () {
    fn to_value(&self) -> Value<'_> {
        Value::Unit
    }
}

impl<'a> IntoValue<'a> for () {
    fn into_value(self) -> Value<'a> {
        Value::Unit
    }
}

impl FromValue<'_> for () {
    fn from_value(_value: Value<'_>) -> Result<Self, FromValueError> {
        // Any value is convertible to unit.
        Ok(())
    }
}

impl TryFrom<Value<'_>> for () {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast()
    }
}

macro_rules! impl_from_to_value_copy {
    ($t:ty: $vt:ident) => {
        impl ToValue for $t {
            fn to_value(&self) -> Value<'_> {
                Value::$vt(*self)
            }
        }

        impl<'a> IntoValue<'a> for $t {
            fn into_value(self) -> Value<'a> {
                Value::$vt(self)
            }
        }

        impl FromValue<'_> for $t {
            fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
                match value {
                    Value::$vt(v) => Ok(v),
                    _ => Err(FromValueError::TypeMismatch {
                        expected: <$t as Reflect>::ty().unwrap().to_string(),
                        actual: value.ty().to_string(),
                    }),
                }
            }
        }
    };
}

impl_reflect!(impl for bool => Bool);
impl_reflect!(impl for i8 => Int8);
impl_reflect!(impl for i16 => Int16);
impl_reflect!(impl for i32 => Int32);
impl_reflect!(impl for i64 => Int64);
impl_reflect!(impl for u8 => UInt8);
impl_reflect!(impl for u16 => UInt16);
impl_reflect!(impl for u32 => UInt32);
impl_reflect!(impl for u64 => UInt64);

impl_from_to_value_copy!(bool: Bool);
impl_from_to_value_copy!(i8: Int8);
impl_from_to_value_copy!(i16: Int16);
impl_from_to_value_copy!(i32: Int32);
impl_from_to_value_copy!(i64: Int64);
impl_from_to_value_copy!(u8: UInt8);
impl_from_to_value_copy!(u16: UInt16);
impl_from_to_value_copy!(u32: UInt32);
impl_from_to_value_copy!(u64: UInt64);

// f32
// ============================================================================
impl_reflect!(impl for f32 => Float32);

impl ToValue for f32 {
    fn to_value(&self) -> Value<'_> {
        self.into_value()
    }
}

impl<'a> IntoValue<'a> for f32 {
    fn into_value(self) -> Value<'a> {
        Value::Float32(OrderedFloat(self))
    }
}

impl FromValue<'_> for f32 {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Float32(v) => Ok(v.0),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a float32".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

// f64
// ============================================================================
impl_reflect!(impl for f64 => Float64);

impl ToValue for f64 {
    fn to_value(&self) -> Value<'_> {
        self.into_value()
    }
}

impl<'a> IntoValue<'a> for f64 {
    fn into_value(self) -> Value<'a> {
        Value::Float64(OrderedFloat(self))
    }
}

impl FromValue<'_> for f64 {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Float64(v) => Ok(v.0),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a float64".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

// PhantomData
// ============================================================================
impl_reflect!(impl<T> for std::marker::PhantomData<T> => Unit);

impl<T> ToValue for std::marker::PhantomData<T> {
    fn to_value(&self) -> Value<'_> {
        self.into_value()
    }
}

impl<'a, T> IntoValue<'a> for std::marker::PhantomData<T> {
    fn into_value(self) -> Value<'a> {
        Value::Unit
    }
}

impl<T> FromValue<'_> for std::marker::PhantomData<T> {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        <()>::try_from(value).map(|()| std::marker::PhantomData)
    }
}

// char
// ============================================================================
impl_reflect!(impl for char => String);

impl ToValue for char {
    fn to_value(&self) -> Value<'_> {
        self.into_value()
    }
}

impl<'a> IntoValue<'a> for char {
    fn into_value(self) -> Value<'a> {
        Value::String(self.to_string().into())
    }
}

impl<'a> FromValue<'a> for char {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        fn make_error<T: std::fmt::Display>(actual: T) -> FromValueError {
            FromValueError::TypeMismatch {
                expected: "a character".to_owned(),
                actual: actual.to_string(),
            }
        }
        fn from_str(s: &str) -> Result<char, FromValueError> {
            let mut chars = s.chars();
            let c = chars.next().ok_or_else(|| make_error("an empty string"))?;
            if chars.next().is_some() {
                return Err(make_error(format!(
                    "a string of size {}",
                    chars.count() + 2
                )));
            }
            Ok(c)
        }
        match value {
            Value::String(s) => from_str(&s),
            Value::ByteString(s) => from_str(std::str::from_utf8(&s)?),
            _ => Err(make_error(value)),
        }
    }
}

// str
// ============================================================================
impl_reflect!(impl for &str => String);

impl ToValue for str {
    fn to_value(&self) -> Value<'_> {
        Value::String(Cow::Borrowed(self))
    }
}

impl<'long: 'short, 'short> FromValue<'long> for &'short str {
    fn from_value(value: Value<'long>) -> Result<Self, FromValueError> {
        match value {
            Value::String(Cow::Borrowed(s)) => Ok(s),
            Value::ByteString(Cow::Borrowed(s)) => Ok(std::str::from_utf8(s)?),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a borrowed string".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

// String
// ============================================================================
impl_reflect!(impl for String => String);

impl ToValue for String {
    fn to_value(&self) -> Value<'_> {
        Value::String(Cow::Borrowed(self))
    }
}

impl<'a> IntoValue<'a> for String {
    fn into_value(self) -> Value<'a> {
        Value::String(Cow::Owned(self))
    }
}

impl FromValue<'_> for String {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::String(s) => Ok(s.into_owned()),
            Value::ByteString(s) => Ok(String::from_utf8(s.into_owned())?),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a String".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

// CStr
// ============================================================================

// CString
// ============================================================================
impl_reflect!(impl for std::ffi::CString => Raw);

impl ToValue for std::ffi::CString {
    fn to_value(&self) -> Value<'_> {
        Value::ByteString(Cow::Borrowed(self.as_bytes()))
    }
}

impl<'a> IntoValue<'a> for std::ffi::CString {
    fn into_value(self) -> Value<'a> {
        Value::ByteString(Cow::Owned(self.as_bytes().to_owned()))
    }
}

impl<'a> FromValue<'a> for std::ffi::CString {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        match value {
            Value::String(s) => Ok(Self::new(s.into_owned())?),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a C String".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

// Option
// ============================================================================
impl<T> Reflect for Option<T>
where
    T: Reflect,
{
    fn ty() -> Option<Type> {
        Some(ty::option(<T as Reflect>::ty()))
    }
}

impl<T> RuntimeReflect for Option<T>
where
    T: RuntimeReflect,
{
    fn ty(&self) -> Type {
        ty::option(self.as_ref().map(RuntimeReflect::ty))
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(&self) -> Value<'_> {
        Value::Option(self.as_ref().map(|v| Box::new(v.to_value())))
    }
}

impl<'a, T> IntoValue<'a> for Option<T>
where
    T: IntoValue<'a>,
{
    fn into_value(self) -> Value<'a> {
        Value::Option(self.map(|v| Box::new(v.into_value())))
    }
}

impl<'a, T> FromValue<'a> for Option<T>
where
    T: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        match value {
            Value::Option(o) => o.map(|v| (*v).cast()).transpose(),
            _ => Err(FromValueError::TypeMismatch {
                expected: "an optional value".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

// Bytes
// ============================================================================
impl_reflect!(impl for Bytes => Raw);

impl ToValue for Bytes {
    fn to_value(&self) -> Value<'_> {
        Value::Raw(Cow::Borrowed(self))
    }
}

impl<'a> IntoValue<'a> for Bytes {
    fn into_value(self) -> Value<'a> {
        Value::Raw(Cow::Owned(self.into()))
    }
}

impl FromValue<'_> for Bytes {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Raw(r) => Ok(Self::copy_from_slice(&r)),
            _ => Err(FromValueError::TypeMismatch {
                expected: "Bytes".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

// Sequences
// ============================================================================
macro_rules! impl_list_value {
    (for T $(: $bound1:ident $(+ $bound2:ident)*)? , $t:ty) => {
        impl<T> Reflect for $t
        where
            T: Reflect,
        {
            fn ty() -> Option<Type> {
                Some(ty::list(<T as Reflect>::ty()))
            }
        }

        impl<T> RuntimeReflect for $t
        where
            T: RuntimeReflect
        {
            fn ty(&self) -> Type {
                let value_type = ty::reduce_type(self.iter().map(RuntimeReflect::ty));
                ty::list(value_type)
            }
        }

        impl<T> ToValue for $t
        where
            T: ToValue $(+ $bound1 $(+ $bound2)*)?
        {
            fn to_value(&self) -> Value<'_> {
                Value::List(self.iter().map(ToValue::to_value).collect())
            }
        }

        impl<'a, T> IntoValue<'a> for $t
        where
            T: IntoValue<'a> $(+ $bound1 $(+ $bound2)*)?
        {
            fn into_value(self) -> Value<'a> {
                Value::List(self.into_iter().map(IntoValue::into_value).collect())
            }
        }

        impl<'a, T> FromValue<'a> for $t
        where
            T: FromValue<'a> $(+ $bound1 $(+ $bound2)*)?
        {
            fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
                match value {
                    Value::List(list) => list.into_iter().map(Value::cast).collect(),
                    _ => Err(FromValueError::TypeMismatch {
                        expected: "a list".to_owned(),
                        actual: value.ty().to_string(),
                    }),
                }
            }
        }
    };
}

impl_list_value!(for T, Vec<T>);
impl_list_value!(for T, std::collections::LinkedList<T>);
impl_list_value!(for T, std::collections::VecDeque<T>);
impl_list_value!(for T: Ord, std::collections::BinaryHeap<T>);
impl_list_value!(for T: Ord, std::collections::BTreeSet<T>);
impl_list_value!(for T: Hash + Eq, std::collections::HashSet<T>);

// Slices
// ============================================================================
impl<T> Reflect for [T]
where
    T: Reflect,
{
    fn ty() -> Option<Type> {
        Some(ty::list(T::ty()))
    }
}

impl<T> RuntimeReflect for [T]
where
    T: RuntimeReflect,
{
    fn ty(&self) -> Type {
        let value_type = ty::reduce_type(self.iter().map(RuntimeReflect::ty));
        ty::list(value_type)
    }
}

impl<T> ToValue for [T]
where
    T: ToValue,
{
    fn to_value(&self) -> Value<'_> {
        Value::List(self.iter().map(ToValue::to_value).collect())
    }
}

impl<'long: 'short, 'short> FromValue<'long> for &'short [u8] {
    fn from_value(value: Value<'long>) -> Result<Self, FromValueError> {
        match value {
            Value::Raw(Cow::Borrowed(r)) => Ok(r),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a slice of bytes".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

// Maps
// ============================================================================
macro_rules! impl_map_value {
    ($t:ty) => {
        impl<K, V> Reflect for $t
        where
            K: Reflect,
            V: Reflect,
        {
            fn ty() -> Option<Type> {
                Some(ty::map(K::ty(), V::ty()))
            }
        }

        impl<K, V> RuntimeReflect for $t
        where
            K: RuntimeReflect,
            V: RuntimeReflect,
        {
            fn ty(&self) -> Type {
                let (key_type, value_type) =
                    ty::reduce_map_types(self.iter().map(|(k, v)| (k.ty(), v.ty())));
                ty::map(key_type, value_type)
            }
        }

        impl<K, V> ToValue for $t
        where
            K: ToValue,
            V: ToValue,
        {
            fn to_value(&self) -> Value<'_> {
                Value::Map(
                    self.iter()
                        .map(|(k, v)| (k.to_value(), v.to_value()))
                        .collect(),
                )
            }
        }

        impl<'a, K, V> IntoValue<'a> for $t
        where
            K: IntoValue<'a>,
            V: IntoValue<'a>,
        {
            fn into_value(self) -> Value<'a> {
                Value::Map(
                    self.into_iter()
                        .map(|(k, v)| (k.into_value(), v.into_value()))
                        .collect(),
                )
            }
        }
    };
}

impl_map_value!(crate::Map<K, V>);
impl_map_value!(std::collections::HashMap<K, V>);
impl_map_value!(std::collections::BTreeMap<K, V>);

macro_rules! from_value_map_expr {
    ($value:ident) => {
        match $value {
            Value::Map(m) => m
                .into_iter()
                .map(|(k, v)| -> Result<(K, V), FromValueError> { Ok((k.cast()?, v.cast()?)) })
                .collect(),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a map".to_owned(),
                actual: $value.ty().to_string(),
            }),
        }
    };
}

impl<'a, K, V> FromValue<'a> for crate::Map<K, V>
where
    K: FromValue<'a> + PartialEq,
    V: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        from_value_map_expr!(value)
    }
}

impl<'a, K, V> FromValue<'a> for std::collections::HashMap<K, V>
where
    K: FromValue<'a> + Eq + Hash,
    V: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        from_value_map_expr!(value)
    }
}

impl<'a, K, V> FromValue<'a> for std::collections::BTreeMap<K, V>
where
    K: FromValue<'a> + Ord,
    V: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        from_value_map_expr!(value)
    }
}

// Arrays
// ============================================================================
impl<T> Reflect for [T; 0] {
    fn ty() -> Option<Type> {
        Some(Type::Unit)
    }
}

impl<T> RuntimeReflect for [T; 0] {
    fn ty(&self) -> Type {
        Type::Unit
    }
}

impl<T> ToValue for [T; 0] {
    fn to_value(&self) -> Value<'_> {
        Value::Unit
    }
}

impl<'a, T> IntoValue<'a> for [T; 0] {
    fn into_value(self) -> Value<'a> {
        Value::Unit
    }
}

impl<T> FromValue<'_> for [T; 0] {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Unit => Ok([]),
            _ => Err(FromValueError::TypeMismatch {
                expected: "an array of size 0".to_owned(),
                actual: value.ty().to_string(),
            }),
        }
    }
}

macro_rules! impl_array_value {
    ($len:literal) => {
        seq!(N in 1..$len {
            #(
                impl<T> Reflect for [T; N]
                where
                    T: Reflect,
                {
                    fn ty() -> Option<Type> {
                        let value_ty = <T as Reflect>::ty();
                        let tuple = ty::Tuple::Tuple(
                            vec![value_ty; N]
                        );
                        Some(Type::Tuple(tuple))
                    }
                }

                impl<T> RuntimeReflect for [T; N]
                where
                    T: RuntimeReflect,
                {
                    fn ty(&self) -> Type {
                        let value_types = self.iter().map(|v| Some(v.ty()));
                        let tuple = ty::Tuple::Tuple(
                            value_types.collect()
                        );
                        Type::Tuple(tuple)
                    }
                }

                impl<T> ToValue for [T; N]
                where
                    T: ToValue
                {
                    fn to_value(&self) -> Value<'_> {
                        Value::Tuple(self.iter().map(ToValue::to_value).collect())
                    }
                }

                impl<'a, T> IntoValue<'a> for [T; N]
                where
                    T: IntoValue<'a>
                {
                    fn into_value(self) -> Value<'a> {
                        Value::Tuple(self.into_iter().map(IntoValue::into_value).collect())
                    }
                }

                impl<'a, T> FromValue<'a> for [T; N]
                where
                    T: FromValue<'a>
                {
                    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
                        match value {
                            Value::Tuple(elements) if elements.len() == N => {
                                let mut iter = elements.into_iter();
                                seq!(I in 0..N {
                                    Ok([
                                        #(
                                            iter.next().unwrap().cast()?,
                                        )*
                                    ])
                                })
                            },
                            _ => Err(FromValueError::TypeMismatch {
                                expected: format!("an array of size {}", N),
                                actual: value.ty().to_string(),
                            })
                        }
                    }
                }
            )*
        });
    }
}

impl_array_value!(32);

// Tuples
// ============================================================================
macro_rules! impl_tuple_value {
    (@impls $len:literal) => {
        seq!(N in 0..$len {
            impl<#(T~N,)*> Reflect for (#(T~N,)*)
            where
                #(
                    T~N: Reflect,
                )*
            {
                fn ty() -> Option<Type> {
                    let tuple = ty::Tuple::Tuple(
                        vec![
                            #(
                                <T~N as Reflect>::ty(),
                            )*
                        ]
                    );
                    Some(Type::Tuple(tuple))
                }
            }

            impl<#(T~N,)*> RuntimeReflect for (#(T~N,)*)
            where
                #(
                    T~N: RuntimeReflect,
                )*
            {
                fn ty(&self) -> Type {
                    let tuple = ty::Tuple::Tuple(
                        vec![
                            #(
                                Some(self.N.ty()),
                            )*
                        ]
                    );
                    Type::Tuple(tuple)
                }
            }

            impl<#(T~N,)*> ToValue for (#(T~N,)*)
            where
                #(
                    T~N: ToValue,
                )*
            {
                fn to_value(&self) -> Value<'_> {
                    let elements = vec![
                        #(
                            self.N.to_value(),
                        )*
                    ];
                    Value::Tuple(elements)
                }
            }

            impl<'a, #(T~N,)*> IntoValue<'a> for (#(T~N,)*)
            where
                #(
                    T~N: IntoValue<'a>,
                )*
            {
                fn into_value(self) -> Value<'a> {
                    let elements = vec![
                        #(
                            self.N.into_value(),
                        )*
                    ];
                    Value::Tuple(elements)
                }
            }

            impl<'a, #(T~N,)*> FromValue<'a> for (#(T~N,)*)
            where
                #(
                    T~N: FromValue<'a>,
                )*
            {
                fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
                    match value {
                        Value::Tuple(elements) if elements.len() == $len => {
                            let mut iter = elements.into_iter();
                            Ok((
                                #(
                                    iter.next().unwrap().cast()?,
                                )*
                            ))
                        }
                        _ => Err(FromValueError::TypeMismatch {
                            expected: format!("a tuple of size {}", $len),
                            actual: value.ty().to_string(),
                        })
                    }
                }
            }
        });
    };
    ($len:literal) => {
        seq!(N in 1..$len {
            #(
                impl_tuple_value!(@impls N);
            )*
        });
    }
}

impl_tuple_value!(32);

// Refs
// ============================================================================
impl<T> Reflect for &T
where
    T: Reflect + ?Sized,
{
    fn ty() -> Option<Type> {
        T::ty()
    }
}

impl<T> RuntimeReflect for &T
where
    T: RuntimeReflect + ?Sized,
{
    fn ty(&self) -> Type {
        (**self).ty()
    }
}

impl<T> ToValue for &T
where
    T: ToValue + ?Sized,
{
    fn to_value(&self) -> Value<'_> {
        (*self).to_value()
    }
}

impl<'long: 'short, 'short, T> IntoValue<'short> for &'long T
where
    T: ToValue + ?Sized,
{
    fn into_value(self) -> Value<'short> {
        self.to_value()
    }
}

// Mutable Refs
// ============================================================================
impl<T> Reflect for &mut T
where
    T: Reflect + ?Sized,
{
    fn ty() -> Option<Type> {
        T::ty()
    }
}

impl<T> RuntimeReflect for &mut T
where
    T: RuntimeReflect + ?Sized,
{
    fn ty(&self) -> Type {
        (**self).ty()
    }
}

impl<T> ToValue for &mut T
where
    T: ToValue + ?Sized,
{
    fn to_value(&self) -> Value<'_> {
        (**self).to_value()
    }
}

impl<'long: 'short, 'short, T> IntoValue<'short> for &'long mut T
where
    T: ToValue + ?Sized,
{
    fn into_value(self) -> Value<'short> {
        (*self).to_value()
    }
}

// Box
// ============================================================================
impl<T> Reflect for Box<T>
where
    T: Reflect + ?Sized,
{
    fn ty() -> Option<Type> {
        T::ty()
    }
}

impl<T> RuntimeReflect for Box<T>
where
    T: RuntimeReflect + ?Sized,
{
    fn ty(&self) -> Type {
        (**self).ty()
    }
}

impl<T> ToValue for Box<T>
where
    T: ToValue + ?Sized,
{
    fn to_value(&self) -> Value<'_> {
        (**self).to_value()
    }
}

impl<'a, T> IntoValue<'a> for Box<T>
where
    T: IntoValue<'a>,
{
    fn into_value(self) -> Value<'a> {
        (*self).into_value()
    }
}

impl<'a, T> FromValue<'a> for Box<T>
where
    T: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        Ok(Box::new(value.cast()?))
    }
}

// Cow
// ============================================================================
impl<T> Reflect for Cow<'_, T>
where
    T: Reflect + ToOwned + ?Sized,
{
    fn ty() -> Option<Type> {
        T::ty()
    }
}

impl<T> RuntimeReflect for Cow<'_, T>
where
    T: RuntimeReflect + ToOwned + ?Sized,
{
    fn ty(&self) -> Type {
        (**self).ty()
    }
}

impl<'long: 'short, 'short, T> IntoValue<'short> for Cow<'long, T>
where
    T: ToOwned + ?Sized,
    &'long T: IntoValue<'short>,
    T::Owned: IntoValue<'short>,
{
    fn into_value(self) -> Value<'short> {
        match self {
            Cow::Owned(v) => v.into_value(),
            Cow::Borrowed(v) => v.into_value(),
        }
    }
}

impl<'long: 'short, 'short, T> FromValue<'long> for Cow<'short, T>
where
    T: ToOwned + ?Sized,
    &'short T: FromValue<'long>,
{
    fn from_value(value: Value<'long>) -> Result<Self, FromValueError> {
        Ok(Cow::Borrowed(value.cast()?))
    }
}

// Rc
// ============================================================================
impl<T> Reflect for Rc<T>
where
    T: Reflect + ?Sized,
{
    fn ty() -> Option<Type> {
        T::ty()
    }
}

impl<T> RuntimeReflect for Rc<T>
where
    T: RuntimeReflect + ?Sized,
{
    fn ty(&self) -> Type {
        (**self).ty()
    }
}

impl<T> ToValue for Rc<T>
where
    T: ToValue + ?Sized,
{
    fn to_value(&self) -> Value<'_> {
        (**self).to_value()
    }
}

impl<'a, T> IntoValue<'a> for Rc<T>
where
    T: ?Sized,
    for<'t> &'t T: IntoValue<'a>,
{
    fn into_value(self) -> Value<'a> {
        (*self).into_value()
    }
}

impl<'a, T> FromValue<'a> for Rc<T>
where
    T: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        Ok(Rc::new(value.cast()?))
    }
}

// Arc
// ============================================================================
impl<T> Reflect for Arc<T>
where
    T: Reflect + ?Sized,
{
    fn ty() -> Option<Type> {
        T::ty()
    }
}

impl<T> RuntimeReflect for Arc<T>
where
    T: RuntimeReflect + ?Sized,
{
    fn ty(&self) -> Type {
        (**self).ty()
    }
}

impl<'a, T> IntoValue<'a> for Arc<T>
where
    T: ?Sized,
    for<'t> &'t T: IntoValue<'a>,
{
    fn into_value(self) -> Value<'a> {
        (*self).into_value()
    }
}

impl<'a, T> FromValue<'a> for Arc<T>
where
    T: FromValue<'a>,
{
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        Ok(Arc::new(value.cast()?))
    }
}

// TODO:
// - NonZeroU8,
// - NonZeroU16,
// - NonZeroU32,
// - NonZeroU64,
// - NonZeroU128,
// - NonZeroUsize,
// - NonZeroI8,
// - NonZeroI16,
// - NonZeroI32,
// - NonZeroI64,
// - NonZeroI128,
// - NonZeroIsize,
// - Result<T, E>
// - Duration
// - SystemTime
// - Path
// - PathBuf
// - OsStr
// - OSString
