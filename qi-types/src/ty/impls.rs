use super::{common_type, list_of, map_of, option_of, DynamicGetType, StaticGetType, Type};
use crate::{Dynamic, List, Map, Raw, Value};

macro_rules! impl_static_type_traits {
    ($nt:tt => $vt:ident, $($tail:tt)*) => {
        impl StaticGetType for $nt {
            fn static_type() -> Type {
                Type::$vt
            }
        }

        impl_static_type_traits!{ $($tail)* }
    };
    () => {}
}

impl_static_type_traits! {
    () => Unit,
    bool => Bool,
    i16 => Int16,
    i32 => Int32,
    i64 => Int64,
    i8 => Int8,
    u16 => UInt16,
    u32 => UInt32,
    u64 => UInt64,
    u8 => UInt8,
    f32 => Float32,
    f64 => Float64,
}

/// A statically typed value is also dynamically typed.
impl<T> DynamicGetType for T
where
    T: StaticGetType,
{
    fn dynamic_type(&self) -> Option<Type> {
        Some(T::static_type())
    }
}

impl StaticGetType for String {
    fn static_type() -> Type {
        Type::String
    }
}

impl StaticGetType for Raw {
    fn static_type() -> Type {
        Type::Raw
    }
}

impl<T> StaticGetType for Option<T>
where
    T: StaticGetType,
{
    fn static_type() -> Type {
        option_of(T::static_type())
    }
}

impl DynamicGetType for Option<Value> {
    fn dynamic_type(&self) -> Option<Type> {
        Some(option_of(
            self.as_ref().and_then(DynamicGetType::dynamic_type),
        ))
    }
}

impl DynamicGetType for Option<Dynamic> {
    fn dynamic_type(&self) -> Option<Type> {
        Some(option_of(
            self.as_ref().and_then(DynamicGetType::dynamic_type),
        ))
    }
}

impl<T> StaticGetType for List<T>
where
    T: StaticGetType,
{
    fn static_type() -> Type {
        list_of(T::static_type())
    }
}

impl DynamicGetType for List<Value> {
    fn dynamic_type(&self) -> Option<Type> {
        let t = self
            .iter()
            .map(|value| value.dynamic_type())
            .reduce(common_type)
            .flatten();
        Some(list_of(t))
    }
}

impl DynamicGetType for List<Dynamic> {
    fn dynamic_type(&self) -> Option<Type> {
        let t = self
            .iter()
            .map(|value| value.dynamic_type())
            .reduce(common_type)
            .flatten();
        Some(list_of(t))
    }
}

impl<K, V> StaticGetType for Map<K, V>
where
    K: StaticGetType,
    V: StaticGetType,
{
    fn static_type() -> crate::Type {
        map_of(Some(K::static_type()), Some(V::static_type()))
    }
}

impl DynamicGetType for Map<Dynamic, Dynamic> {
    fn dynamic_type(&self) -> Option<Type> {
        self.get_dynamic_type()
    }
}

impl DynamicGetType for Map<Dynamic, Value> {
    fn dynamic_type(&self) -> Option<Type> {
        self.get_dynamic_type()
    }
}

impl DynamicGetType for Map<Value, Dynamic> {
    fn dynamic_type(&self) -> Option<Type> {
        self.get_dynamic_type()
    }
}

impl DynamicGetType for Map<Value, Value> {
    fn dynamic_type(&self) -> Option<Type> {
        self.get_dynamic_type()
    }
}
