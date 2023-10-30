use super::{AsValue, FromValue, FromValueError, Value};
use crate::{ty, Reflect, Type};
use bytes::Bytes;
use ordered_float::OrderedFloat;
use seq_macro::seq;
use std::{borrow::Cow, hash::Hash, rc::Rc, sync::Arc};

macro_rules! impl_reflect {
    ($nt:ty: $vt:ident, $($tail:tt)*) => {
        impl Reflect for $nt {
            fn ty() -> Option<Type> {
                Some(Type::$vt)
            }
        }

        impl_reflect!{ $($tail)* }
    };
    () => {}
}

impl_reflect! {
    (): Unit,
    bool: Bool,
    i8: Int8,
    i16: Int16,
    i32: Int32,
    i64: Int64,
    u8: UInt8,
    u16: UInt16,
    u32: UInt32,
    u64: UInt64,
    f32: Float32,
    f64: Float64,
    String: String,
    &str: String,
    char: String,
    std::ffi::CString: Raw,
}

impl AsValue for () {
    fn value_type(&self) -> Type {
        Type::Unit
    }

    fn as_value(&self) -> Value<'_> {
        Value::Unit
    }
}

impl FromValue<'_> for () {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Unit => Ok(()),
            _ => Err(FromValueError::value_type_mismatch::<Self>(&value)),
        }
    }
}

impl From<()> for Value<'_> {
    fn from((): ()) -> Self {
        Self::Unit
    }
}

impl TryFrom<Value<'_>> for () {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast()
    }
}

macro_rules! impl_valuable_from_to_immediate {
    ($($t:ty: $vt:ident),*) => {
        $(
            impl AsValue for $t {
                fn value_type(&self) -> Type {
                    Type::$vt
                }

                fn as_value(&self) -> Value<'_> {
                    Value::$vt(*self)
                }
            }

            impl FromValue<'_> for $t {
                fn from_value(value: Value<'_>) -> Result<Self, FromValueError>
                {
                    match value {
                        Value::$vt(v) => Ok(v),
                        _ => Err(FromValueError::value_type_mismatch::<Self>(&value))
                    }
                }
            }

            impl From<$t> for Value<'_> {
                fn from(v: $t) -> Self {
                    Self::$vt(v)
                }
            }

            impl TryFrom<Value<'_>> for $t {
                type Error = FromValueError;
                fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
                    value.cast()
                }
            }
        )*
    };
}

impl_valuable_from_to_immediate! {
    bool: Bool,
    i8: Int8,
    i16: Int16,
    i32: Int32,
    i64: Int64,
    u8: UInt8,
    u16: UInt16,
    u32: UInt32,
    u64: UInt64
}

// f32
// ============================================================================
impl AsValue for f32 {
    fn value_type(&self) -> Type {
        Type::Float32
    }

    fn as_value(&self) -> Value<'_> {
        Value::Float32(OrderedFloat(*self))
    }
}

impl FromValue<'_> for f32 {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Float32(v) => Ok(v.0),
            _ => Err(FromValueError::value_type_mismatch::<Self>(&value)),
        }
    }
}

impl From<f32> for Value<'_> {
    fn from(value: f32) -> Self {
        Value::Float32(OrderedFloat(value))
    }
}

impl TryFrom<Value<'_>> for f32 {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast()
    }
}

// f64
// ============================================================================
impl AsValue for f64 {
    fn value_type(&self) -> Type {
        Type::Float64
    }

    fn as_value(&self) -> Value<'_> {
        Value::Float64(OrderedFloat(*self))
    }
}

impl FromValue<'_> for f64 {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Float64(v) => Ok(v.0),
            _ => Err(FromValueError::value_type_mismatch::<Self>(&value)),
        }
    }
}

impl From<f64> for Value<'_> {
    fn from(value: f64) -> Self {
        Value::Float64(OrderedFloat(value))
    }
}

impl TryFrom<Value<'_>> for f64 {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast()
    }
}

// PhantomData
// ============================================================================
impl<T> Reflect for std::marker::PhantomData<T> {
    fn ty() -> Option<Type> {
        Some(Type::Unit)
    }
}

impl<T> AsValue for std::marker::PhantomData<T> {
    fn value_type(&self) -> Type {
        Type::Unit
    }

    fn as_value(&self) -> Value<'_> {
        Value::Unit
    }
}

impl<T> FromValue<'_> for std::marker::PhantomData<T> {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        <()>::try_from(value).map(|()| std::marker::PhantomData)
    }
}

impl<T> From<std::marker::PhantomData<T>> for Value<'_> {
    fn from(_value: std::marker::PhantomData<T>) -> Self {
        Self::Unit
    }
}

impl<T> TryFrom<Value<'_>> for std::marker::PhantomData<T> {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast()
    }
}

// String
// ============================================================================
impl AsValue for String {
    fn value_type(&self) -> Type {
        Type::String
    }

    fn as_value(&self) -> Value<'_> {
        Value::String(Cow::Borrowed(self))
    }
}

impl FromValue<'_> for String {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::String(s) => Ok(s.into_owned()),
            _ => Err(FromValueError::value_type_mismatch::<Self>(&value)),
        }
    }
}

impl From<String> for Value<'_> {
    fn from(value: String) -> Self {
        Self::String(Cow::Owned(value))
    }
}

impl TryFrom<Value<'_>> for String {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast()
    }
}

// str
// ============================================================================
impl AsValue for str {
    fn value_type(&self) -> Type {
        Type::String
    }

    fn as_value(&self) -> Value<'_> {
        Value::String(Cow::Borrowed(self))
    }
}

impl<'a> FromValue<'a> for &'a str {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        match value {
            Value::String(Cow::Borrowed(s)) => Ok(s),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a borrowed string".to_owned(),
                actual: value.value_type().to_string(),
            }),
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(Cow::Borrowed(value))
    }
}

impl<'a> TryFrom<Value<'a>> for &'a str {
    type Error = FromValueError;

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        value.cast()
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

impl<T> AsValue for Option<T>
where
    T: AsValue,
{
    fn value_type(&self) -> Type {
        ty::option(self.as_ref().map(AsValue::value_type))
    }

    fn as_value(&self) -> Value<'_> {
        Value::Option(self.as_ref().map(|v| Box::new(v.as_value())))
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
                actual: value.value_type().to_string(),
            }),
        }
    }
}

impl<'a, T> From<Option<T>> for Value<'a>
where
    T: Into<Value<'a>>,
{
    fn from(o: Option<T>) -> Self {
        Self::Option(o.map(|v| Box::new(v.into())))
    }
}

// Bytes
// ============================================================================
impl Reflect for Bytes {
    fn ty() -> Option<Type> {
        Some(Type::Raw)
    }
}

impl AsValue for Bytes {
    fn value_type(&self) -> Type {
        Type::Raw
    }

    fn as_value(&self) -> Value<'_> {
        Value::Raw(Cow::Borrowed(self))
    }
}

impl FromValue<'_> for Bytes {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Raw(r) => Ok(Self::copy_from_slice(&r)),
            _ => Err(FromValueError::value_type_mismatch::<Self>(&value)),
        }
    }
}

impl From<Bytes> for Value<'_> {
    fn from(value: Bytes) -> Self {
        Self::Raw(Cow::Owned(value.into()))
    }
}

impl TryFrom<Value<'_>> for Bytes {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast()
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

        impl<T> AsValue for $t
        where
            T: AsValue,
        {
            fn value_type(&self) -> Type {
                let value_type = ty::reduce_type(self.iter().map(AsValue::value_type));
                ty::list(value_type)
            }

            fn as_value(&self) -> Value<'_> {
                Value::List(self.iter().map(|v| v.as_value()).collect())
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
                        actual: value.value_type().to_string(),
                    }),
                }
            }
        }

        impl<'a, T> From<$t> for Value<'a>
        where
            T: Into<Value<'a>>,
        {
            fn from(value: $t) -> Self {
                Self::List(value.into_iter().map(Into::into).collect())
            }
        }

        impl<'a, T> TryFrom<Value<'a>> for $t
        where
            T: FromValue<'a> $(+ $bound1 $(+ $bound2)*)?
        {
            type Error = FromValueError;

            fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
                value.cast()
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

impl<T> AsValue for [T]
where
    T: AsValue,
{
    fn value_type(&self) -> Type {
        let value_type = ty::reduce_type(self.iter().map(AsValue::value_type));
        ty::list(value_type)
    }

    fn as_value(&self) -> Value<'_> {
        Value::List(self.iter().map(AsValue::as_value).collect())
    }
}

impl<'a, T> From<&'a [T]> for Value<'a>
where
    T: AsValue,
{
    fn from(value: &'a [T]) -> Self {
        Value::List(value.iter().map(AsValue::as_value).collect())
    }
}

impl<'a> FromValue<'a> for &'a [u8] {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        match value {
            Value::Raw(Cow::Borrowed(r)) => Ok(r),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a slice of bytes".to_owned(),
                actual: value.value_type().to_string(),
            }),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for &'a [u8] {
    type Error = FromValueError;

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        value.cast()
    }
}

// Maps
// ============================================================================
macro_rules! impl_map_value {
    (<$k:ident, $v:ident>: $($t:ty),+) => {
        $(
            impl<$k, $v> Reflect for $t
            where
                $k: Reflect,
                $v: Reflect,
            {
                fn ty() -> Option<Type> {
                    Some(ty::map($k::ty(), $v::ty()))
                }
            }

            impl<$k, $v> AsValue for $t
            where
                $k: AsValue,
                $v: AsValue,
            {
                fn value_type(&self) -> Type {
                    let (key_type, value_type)
                        = ty::reduce_map_types(
                            self.iter().map(|(k, v)| (k.value_type(), v.value_type()))
                        );
                    ty::map(key_type, value_type)
                }

                fn as_value(&self) -> Value<'_> {
                    Value::Map(
                        self.iter()
                            .map(|(k, v)| (k.as_value(), v.as_value()))
                            .collect()
                    )
                }
            }

            impl<'a, $k, $v> From<$t> for Value<'a>
            where
                $k: Into<Value<'a>>,
                $v: Into<Value<'a>>,
            {
                fn from(value: $t) -> Self {
                    Self::Map(
                        value.into_iter()
                            .map(|(k, v)| (k.into(), v.into()))
                            .collect()
                    )
                }
            }
        )+
    };
}

impl_map_value! {
    <K, V>:
        crate::Map<K, V>,
        std::collections::HashMap<K, V>,
        std::collections::BTreeMap<K, V>
}

macro_rules! from_value_map_expr {
    ($value:ident) => {
        match $value {
            Value::Map(m) => m
                .into_iter()
                .map(|(k, v)| -> Result<(K, V), FromValueError> { Ok((k.cast()?, v.cast()?)) })
                .collect(),
            _ => Err(FromValueError::TypeMismatch {
                expected: "a map".to_owned(),
                actual: $value.value_type().to_string(),
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

impl<'a, K, V> TryFrom<Value<'a>> for crate::Map<K, V>
where
    K: FromValue<'a> + PartialEq,
    V: FromValue<'a>,
{
    type Error = FromValueError;
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        value.cast()
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

impl<'a, K, V> TryFrom<Value<'a>> for std::collections::HashMap<K, V>
where
    K: FromValue<'a> + Eq + Hash,
    V: FromValue<'a>,
{
    type Error = FromValueError;
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        value.cast()
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

impl<'a, K, V> TryFrom<Value<'a>> for std::collections::BTreeMap<K, V>
where
    K: FromValue<'a> + Ord,
    V: FromValue<'a>,
{
    type Error = FromValueError;
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        value.cast()
    }
}

// Arrays
// ============================================================================

// Does not require T: Reflect.
impl<T> Reflect for [T; 0] {
    fn ty() -> Option<Type> {
        Some(Type::Unit)
    }
}

impl<T> AsValue for [T; 0] {
    fn value_type(&self) -> Type {
        Type::Unit
    }

    fn as_value(&self) -> Value<'_> {
        Value::Unit
    }
}

impl<T> FromValue<'_> for [T; 0] {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        match value {
            Value::Unit => Ok([]),
            _ => Err(FromValueError::value_type_mismatch::<Self>(&value)),
        }
    }
}

impl<T> From<[T; 0]> for Value<'_> {
    fn from(_value: [T; 0]) -> Self {
        Self::Unit
    }
}

impl<T> TryFrom<Value<'_>> for [T; 0] {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast()
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
                        Some(ty::Type::Tuple(tuple))
                    }
                }

                impl<T> AsValue for [T; N]
                where
                    T: AsValue
                {
                    fn value_type(&self) -> Type {
                        let tuple = ty::Tuple::Tuple(
                            self.iter().map(AsValue::value_type).map(Some).collect()
                        );
                        Type::Tuple(tuple)
                    }

                    fn as_value(&self) -> Value<'_> {
                        Value::Tuple(self.iter().map(AsValue::as_value).collect())
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
                                actual: value.value_type().to_string(),
                            })
                        }
                    }
                }

                impl<'a, T> From<[T; N]> for Value<'a>
                where
                    T: Into<Value<'a>>
                {
                    fn from(value: [T; N]) -> Self {
                        Self::Tuple(value.into_iter().map(Into::into).collect())
                    }
                }

                impl<'a, T> TryFrom<Value<'a>> for [T; N]
                where
                    T: FromValue<'a>
                {
                    type Error = FromValueError;

                    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
                        value.cast()
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
                    Some(ty::Type::Tuple(tuple))
                }
            }

            impl<#(T~N,)*> AsValue for (#(T~N,)*)
            where
                #(
                    T~N: AsValue,
                )*
            {
                fn value_type(&self) -> Type {
                    let tuple = ty::Tuple::Tuple(
                        vec![
                            #(
                                Some(self.N.value_type()),
                            )*
                        ]
                    );
                    Type::Tuple(tuple)
                }

                fn as_value(&self) -> Value<'_> {
                    let elements = vec![
                        #(
                            self.N.as_value(),
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
                            actual: value.value_type().to_string(),
                        })
                    }
                }
            }

            impl<'a, #(T~N,)*> From<(#(T~N,)*)> for Value<'a>
            where
                #(
                    T~N: Into<Value<'a>>,
                )*
            {
                fn from(value: (#(T~N,)*)) -> Self {
                    let elements = vec![
                        #(
                            value.N.into(),
                        )*
                    ];
                    Self::Tuple(elements)
                }
            }

            impl<'a, #(T~N,)*> TryFrom<Value<'a>> for (#(T~N,)*)
            where
                #(
                    T~N: FromValue<'a>,
                )*
            {
                type Error = FromValueError;

                fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
                    value.cast()
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

// macro_rules! nonzero_integers {
//     ($($T:ident,)+) => {
//         $(
//             impl Serialize for num::$T {
//                 fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//                 where
//                     S: Serializer,
//                 {
//                     self.get().serialize(serializer)
//                 }
//             }
//         )+
//     }
// }

// nonzero_integers! {
//     NonZeroU8,
//     NonZeroU16,
//     NonZeroU32,
//     NonZeroU64,
//     NonZeroU128,
//     NonZeroUsize,
// }

// #[cfg(not(no_num_nonzero_signed))]
// nonzero_integers! {
//     NonZeroI8,
//     NonZeroI16,
//     NonZeroI32,
//     NonZeroI64,
//     NonZeroI128,
//     NonZeroIsize,
// }

// ////////////////////////////////////////////////////////////////////////////////

// impl<T, E> Serialize for Result<T, E>
// where
//     T: Serialize,
//     E: Serialize,
// {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         match *self {
//             Result::Ok(ref value) => serializer.serialize_newtype_variant("Result", 0, "Ok", value),
//             Result::Err(ref value) => {
//                 serializer.serialize_newtype_variant("Result", 1, "Err", value)
//             }
//         }
//     }
// }

// ////////////////////////////////////////////////////////////////////////////////

// impl Serialize for Duration {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         use super::SerializeStruct;
//         let mut state = tri!(serializer.serialize_struct("Duration", 2));
//         tri!(state.serialize_field("secs", &self.as_secs()));
//         tri!(state.serialize_field("nanos", &self.subsec_nanos()));
//         state.end()
//     }
// }

// ////////////////////////////////////////////////////////////////////////////////

// #[cfg(feature = "std")]
// impl Serialize for SystemTime {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         use super::SerializeStruct;
//         let duration_since_epoch = match self.duration_since(UNIX_EPOCH) {
//             Ok(duration_since_epoch) => duration_since_epoch,
//             Err(_) => return Err(S::Error::custom("SystemTime must be later than UNIX_EPOCH")),
//         };
//         let mut state = tri!(serializer.serialize_struct("SystemTime", 2));
//         tri!(state.serialize_field("secs_since_epoch", &duration_since_epoch.as_secs()));
//         tri!(state.serialize_field("nanos_since_epoch", &duration_since_epoch.subsec_nanos()));
//         state.end()
//     }
// }

// ////////////////////////////////////////////////////////////////////////////////

// ////////////////////////////////////////////////////////////////////////////////

// #[cfg(feature = "std")]
// impl Serialize for Path {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         match self.to_str() {
//             Some(s) => s.serialize(serializer),
//             None => Err(Error::custom("path contains invalid UTF-8 characters")),
//         }
//     }
// }

// #[cfg(feature = "std")]
// impl Serialize for PathBuf {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         self.as_path().serialize(serializer)
//     }
// }

// #[cfg(all(feature = "std", any(unix, windows)))]
// impl Serialize for OsStr {
//     #[cfg(unix)]
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         use std::os::unix::ffi::OsStrExt;
//         serializer.serialize_newtype_variant("OsString", 0, "Unix", self.as_bytes())
//     }

//     #[cfg(windows)]
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         use std::os::windows::ffi::OsStrExt;
//         let val = self.encode_wide().collect::<Vec<_>>();
//         serializer.serialize_newtype_variant("OsString", 1, "Windows", &val)
//     }
// }

// #[cfg(all(feature = "std", any(unix, windows)))]
// impl Serialize for OsString {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         self.as_os_str().serialize(serializer)
//     }
// }

// Pointers
// ============================================================================
macro_rules! impl_deref_as_value {
    ($($desc:tt)+) => {
        impl $($desc)+ {
            fn value_type(&self) -> Type {
                (**self).value_type()
            }

            fn as_value(&self) -> Value<'_> {
                (**self).as_value()
            }
        }
    }
}

impl_deref_as_value!(<T> AsValue for &T where T: AsValue + ?Sized);
impl_deref_as_value!(<T> AsValue for &mut T where T: AsValue + ?Sized);
impl_deref_as_value!(<T> AsValue for Box<T> where T: AsValue + ?Sized);
impl_deref_as_value!(<'a, T> AsValue for Cow<'a, T> where T: AsValue + ToOwned + ?Sized);
impl_deref_as_value!(<T> AsValue for Rc<T> where T: AsValue + ?Sized);
impl_deref_as_value!(<T> AsValue for Arc<T> where T: AsValue + ?Sized);

// Value
// ============================================================================
impl AsValue for Value<'_> {
    fn value_type(&self) -> Type {
        match self {
            Self::Unit => Type::Unit,
            Self::Bool(_) => Type::Bool,
            Self::Int8(_) => Type::Int8,
            Self::UInt8(_) => Type::UInt8,
            Self::Int16(_) => Type::Int16,
            Self::UInt16(_) => Type::UInt16,
            Self::Int32(_) => Type::Int32,
            Self::UInt32(_) => Type::UInt32,
            Self::Int64(_) => Type::Int64,
            Self::UInt64(_) => Type::UInt64,
            Self::Float32(_) => Type::Float32,
            Self::Float64(_) => Type::Float64,
            Self::String(_) => Type::String,
            Self::Raw(_) => Type::Raw,
            Self::Option(v) => v.value_type(),
            Self::List(v) => v.value_type(),
            Self::Map(v) => v.value_type(),
            Self::Tuple(v) => Type::Tuple(ty::Tuple::Tuple(
                v.iter().map(AsValue::value_type).map(Some).collect(),
            )),
            Self::Object(_) => Type::Object,
            Self::Dynamic(v) => v.value_type(),
        }
    }

    fn as_value(&self) -> Value<'_> {
        self.clone()
    }
}

impl<'a> FromValue<'a> for Value<'a> {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        Ok(value)
    }
}
