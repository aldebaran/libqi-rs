use crate::{
    list_ty, map_ty,
    num_bool::*,
    option_ty,
    tuple::*,
    ty::{self, Type},
    FormatterExt, List, Map, Object, Raw, Signature, Value,
};

/// [`Dynamic`] represents a `dynamic` value in the `qi` type system.
///
/// It is a value associated with its type information.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, derive_more::From, derive_more::TryInto)]
pub enum Dynamic {
    #[from]
    Unit,
    #[from]
    Bool(bool),
    #[from(forward)]
    Number(Number),
    #[from]
    String(String),
    #[from]
    Raw(Raw),
    #[from]
    Option(OptionDynamic),
    #[from]
    List(ListDynamic),
    #[from]
    Map(MapDynamic),
    #[from]
    Tuple(TupleDynamic),
    Object(Box<Object>),
    #[try_into(ignore)]
    Dynamic(Box<Dynamic>),
}

impl Dynamic {
    pub fn new(value: Value, t: Option<Type>) -> Result<Self, TypeMismatchError> {
        use ty::DynamicGetType;
        if !value.has_type(t.as_ref()) {
            return Err(TypeMismatchError {
                expected: t,
                actual: value.ty(),
            });
        }
        let value = match value {
            Value::Unit => Self::Unit,
            Value::Bool(b) => Self::Bool(b),
            Value::Number(n) => Self::Number(n),
            Value::String(s) => Self::String(s),
            Value::Raw(r) => Self::Raw(r),
            Value::Object(o) => Self::Object(o),
            Value::Dynamic(d) => Self::Dynamic(d),
            Value::Option(option) => {
                let value_type = match t {
                    Some(Type::Option(o)) => o.as_deref().cloned(),
                    _ => unreachable!(),
                };
                Self::Option(OptionDynamic(*option, value_type))
            }
            Value::List(list) => {
                let value_type = match t {
                    Some(Type::List(l)) => l.as_deref().cloned(),
                    _ => unreachable!(),
                };
                Self::List(ListDynamic(list, value_type))
            }
            Value::Map(map) => {
                let (key_type, value_type) = match t {
                    Some(Type::Map { key, value }) => {
                        (key.as_deref().cloned(), value.as_deref().cloned())
                    }
                    _ => unreachable!(),
                };
                Self::Map(MapDynamic {
                    value: map,
                    key_type,
                    value_type,
                })
            }
            Value::Tuple(tuple) => {
                let tuple_type = match t {
                    Some(Type::Tuple(tuple_type)) => tuple_type,
                    _ => unreachable!(),
                };
                Self::Tuple(TupleDynamic(tuple, tuple_type))
            }
        };
        Ok(value)
    }

    pub fn from_value(value: Value) -> Self {
        use ty::DynamicGetType;
        let t = value.ty();
        Self::new(value, t).unwrap()
    }

    pub fn as_unit(&self) -> Option<()> {
        match self {
            Dynamic::Unit => Some(()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Dynamic::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<Number> {
        match self {
            Dynamic::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&String> {
        match self {
            Dynamic::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_string(self) -> Option<String> {
        match self {
            Dynamic::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_raw(&self) -> Option<&Raw> {
        match self {
            Dynamic::Raw(r) => Some(r),
            _ => None,
        }
    }

    pub fn into_raw(self) -> Option<Raw> {
        match self {
            Dynamic::Raw(r) => Some(r),
            _ => None,
        }
    }

    pub fn as_option(&self) -> Option<&Option<Value>> {
        match self {
            Dynamic::Option(o) => Some(&o.0),
            _ => None,
        }
    }

    pub fn into_option(self) -> Option<Option<Value>> {
        match self {
            Dynamic::Option(o) => Some(o.0),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&List<Value>> {
        match self {
            Dynamic::List(l) => Some(&l.0),
            _ => None,
        }
    }

    pub fn into_list(self) -> Option<List<Value>> {
        match self {
            Dynamic::List(l) => Some(l.0),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&Map<Value, Value>> {
        match self {
            Dynamic::Map(m) => Some(&m.value),
            _ => None,
        }
    }

    pub fn into_map(self) -> Option<Map<Value, Value>> {
        match self {
            Dynamic::Map(m) => Some(m.value),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> Option<&Tuple> {
        match self {
            Dynamic::Tuple(t) => Some(&t.0),
            _ => None,
        }
    }

    pub fn into_tuple(self) -> Option<Tuple> {
        match self {
            Dynamic::Tuple(t) => Some(t.0),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Dynamic::Object(o) => Some(o.as_ref()),
            _ => None,
        }
    }

    pub fn into_object(self) -> Option<Object> {
        match self {
            Dynamic::Object(o) => Some(*o),
            _ => None,
        }
    }

    pub fn as_inner_dynamic(&self) -> Option<&Dynamic> {
        match self {
            Dynamic::Dynamic(d) => Some(d),
            _ => None,
        }
    }

    pub fn into_inner_dynamic(self) -> Option<Dynamic> {
        match self {
            Dynamic::Dynamic(d) => Some(*d),
            _ => None,
        }
    }

    pub fn into_value(self) -> Value {
        match self {
            Self::Unit => Value::Unit,
            Self::Bool(b) => Value::Bool(b),
            Self::Number(n) => Value::Number(n),
            Self::String(s) => Value::String(s),
            Self::Raw(r) => Value::Raw(r),
            Self::Option(o) => o.into_value(),
            Self::List(l) => l.into_value(),
            Self::Map(m) => m.into_value(),
            Self::Tuple(t) => t.into_value(),
            Self::Object(o) => Value::Object(o),
            Self::Dynamic(d) => Value::Dynamic(d),
        }
    }
}

impl Default for Dynamic {
    fn default() -> Self {
        Self::Unit
    }
}

impl From<&str> for Dynamic {
    fn from(s: &str) -> Self {
        Dynamic::String(s.to_owned())
    }
}

impl From<Option<Value>> for Dynamic {
    fn from(v: Option<Value>) -> Self {
        Self::Option(OptionDynamic::from(v))
    }
}

impl From<Object> for Dynamic {
    fn from(v: Object) -> Self {
        Self::Object(Box::new(v))
    }
}

impl ty::DynamicGetType for Dynamic {
    fn ty(&self) -> Option<Type> {
        None
    }

    fn deep_ty(&self) -> Type {
        match self {
            Self::Unit => ().deep_ty(),
            Self::Bool(b) => b.deep_ty(),
            Self::Number(n) => n.deep_ty(),
            Self::String(s) => s.deep_ty(),
            Self::Raw(r) => r.deep_ty(),
            Self::Option(o) => o.deep_ty(),
            Self::List(l) => l.deep_ty(),
            Self::Map(m) => m.deep_ty(),
            Self::Tuple(t) => t.deep_ty(),
            Self::Object(o) => o.deep_ty(),
            Self::Dynamic(d) => d.deep_ty(),
        }
    }
}

impl std::fmt::Display for Dynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Unit => f.write_str("()"),
            Self::Bool(b) => b.fmt(f),
            Self::Number(n) => n.fmt(f),
            Self::String(s) => s.fmt(f),
            Self::Raw(r) => f.write_raw(r),
            Self::Option(o) => o.fmt(f),
            Self::List(l) => l.fmt(f),
            Self::Map(m) => m.fmt(f),
            Self::Tuple(t) => t.fmt(f),
            Self::Object(o) => o.fmt(f),
            Self::Dynamic(d) => d.fmt(f),
        }
    }
}

fn serialize_signed_value<S, T, V>(serializer: S, t: T, value: &V) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
    T: Into<Option<Type>>,
    V: serde::Serialize,
{
    use serde::ser::SerializeTuple;
    let mut serializer = serializer.serialize_tuple(2)?;
    serializer.serialize_element(&Signature::new(t.into()))?;
    serializer.serialize_element(value)?;
    serializer.end()
}

impl serde::Serialize for Dynamic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use ty::DynamicGetType;
        match self {
            Self::Unit => serialize_signed_value(serializer, ().ty(), &()),
            Self::Bool(b) => serialize_signed_value(serializer, b.ty(), b),
            Self::Number(n) => serialize_signed_value(serializer, n.ty(), n),
            Self::String(s) => serialize_signed_value(serializer, s.ty(), s),
            Self::Raw(r) => serialize_signed_value(serializer, r.ty(), r),
            Self::Option(o) => o.serialize(serializer),
            Self::List(l) => l.serialize(serializer),
            Self::Map(m) => m.serialize(serializer),
            Self::Tuple(t) => t.serialize(serializer),
            Self::Object(o) => serialize_signed_value(serializer, o.ty(), o.as_ref()),
            Self::Dynamic(d) => serialize_signed_value(serializer, d.ty(), d.as_ref()),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Dynamic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Dynamic;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a dynamic value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de;

                let invalid_length =
                    |i| de::Error::invalid_length(i, &"a sequence of size 2 (signature, value)");

                // Signature
                let signature: Signature = seq.next_element()?.ok_or_else(|| invalid_length(0))?;
                let value_type = signature.into_type();

                // Value
                let value = seq
                    .next_element_seed(DynamicSeed(value_type))?
                    .ok_or_else(|| invalid_length(1))?;

                Ok(value)
            }
        }

        deserializer.deserialize_tuple(2, Visitor)
    }
}

struct DynamicSeed(Option<Type>);

impl<'de> serde::de::DeserializeSeed<'de> for DynamicSeed {
    type Value = Dynamic;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        let value = match self.0 {
            Some(t) => match t {
                Type::Unit => {
                    <()>::deserialize(deserializer)?;
                    Dynamic::Unit
                }
                Type::Bool => {
                    let v = bool::deserialize(deserializer)?;
                    Dynamic::Bool(v)
                }
                Type::Int8 => {
                    let v = i8::deserialize(deserializer)?;
                    Dynamic::Number(Number::Int8(v))
                }
                Type::UInt8 => {
                    let v = u8::deserialize(deserializer)?;
                    Dynamic::Number(Number::UInt8(v))
                }
                Type::Int16 => {
                    let v = i16::deserialize(deserializer)?;
                    Dynamic::Number(Number::Int16(v))
                }
                Type::UInt16 => {
                    let v = u16::deserialize(deserializer)?;
                    Dynamic::Number(Number::UInt16(v))
                }
                Type::Int32 => {
                    let v = i32::deserialize(deserializer)?;
                    Dynamic::Number(Number::Int32(v))
                }
                Type::UInt32 => {
                    let v = u32::deserialize(deserializer)?;
                    Dynamic::Number(Number::UInt32(v))
                }
                Type::Int64 => {
                    let v = i64::deserialize(deserializer)?;
                    Dynamic::Number(Number::Int64(v))
                }
                Type::UInt64 => {
                    let v = u64::deserialize(deserializer)?;
                    Dynamic::Number(Number::UInt64(v))
                }
                Type::Float32 => {
                    let v = Float32::deserialize(deserializer)?;
                    Dynamic::Number(Number::Float32(v))
                }
                Type::Float64 => {
                    let v = Float64::deserialize(deserializer)?;
                    Dynamic::Number(Number::Float64(v))
                }
                Type::String => {
                    let v = String::deserialize(deserializer)?;
                    Dynamic::String(v)
                }
                Type::Raw => {
                    let v = Raw::deserialize(deserializer)?;
                    Dynamic::Raw(v)
                }
                Type::Object => {
                    let v = Object::deserialize(deserializer)?;
                    Dynamic::Object(Box::new(v))
                }
                Type::Option(t) => {
                    let v = OptionDynamicSeed(t.as_deref().cloned()).deserialize(deserializer)?;
                    Dynamic::Option(v)
                }
                Type::List(t) | Type::VarArgs(t) => {
                    let v = ListDynamicSeed(t.as_deref().cloned()).deserialize(deserializer)?;
                    Dynamic::List(v)
                }
                Type::Map { key, value } => {
                    let v = MapDynamicSeed {
                        key: key.as_deref().cloned(),
                        value: value.as_deref().cloned(),
                    }
                    .deserialize(deserializer)?;
                    Dynamic::Map(v)
                }
                Type::Tuple(tuple) => {
                    let v = TupleDynamicSeed(tuple).deserialize(deserializer)?;
                    Dynamic::Tuple(v)
                }
            },
            None => {
                let v = Dynamic::deserialize(deserializer)?;
                Dynamic::Dynamic(Box::new(v))
            }
        };
        Ok(value)
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct OptionDynamic(Option<Value>, Option<Type>);

impl OptionDynamic {
    pub fn into_value(self) -> Value {
        Value::Option(Box::new(self.0))
    }

    pub fn into_option(self) -> Option<Value> {
        self.0
    }

    fn ty(&self) -> Type {
        Type::Option(self.1.clone().map(Box::new))
    }
}

impl ty::DynamicGetType for OptionDynamic {
    fn ty(&self) -> Option<Type> {
        Some(self.ty())
    }

    fn deep_ty(&self) -> Type {
        self.ty()
    }
}

impl std::fmt::Display for OptionDynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_option(&self.0)
    }
}

impl From<Option<Value>> for OptionDynamic {
    fn from(v: Option<Value>) -> Self {
        let ty = v.as_ref().and_then(ty::DynamicGetType::ty);
        Self(v, ty)
    }
}

impl From<OptionDynamic> for Value {
    fn from(o: OptionDynamic) -> Self {
        o.into_value()
    }
}

impl From<OptionDynamic> for Option<Value> {
    fn from(o: OptionDynamic) -> Self {
        o.into_option()
    }
}

impl serde::Serialize for OptionDynamic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_signed_value(serializer, option_ty!(self.1.clone()), &self.0)
    }
}

struct OptionDynamicSeed(Option<Type>);

impl<'de> serde::de::DeserializeSeed<'de> for OptionDynamicSeed {
    type Value = OptionDynamic;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor(Option<Type>);
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = OptionDynamic;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an optional value")
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::de::DeserializeSeed;
                let typed_value = DynamicSeed(self.0.clone()).deserialize(deserializer)?;
                // Drop the type information and transform into a simple value. The type
                // information is already stored with the optional.
                let value = typed_value.into_value();
                Ok(OptionDynamic(Some(value), self.0))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OptionDynamic(None, self.0))
            }
        }
        deserializer.deserialize_option(Visitor(self.0))
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ListDynamic(List<Value>, Option<Type>);

impl ListDynamic {
    pub fn into_value(self) -> Value {
        Value::List(self.0)
    }

    pub fn into_list(self) -> List<Value> {
        self.0
    }

    fn ty(&self) -> Type {
        Type::List(self.1.clone().map(Box::new))
    }
}

impl ty::DynamicGetType for ListDynamic {
    fn ty(&self) -> Option<Type> {
        Some(self.ty())
    }

    fn deep_ty(&self) -> Type {
        self.ty()
    }
}

impl From<ListDynamic> for Value {
    fn from(l: ListDynamic) -> Self {
        l.into_value()
    }
}

impl From<ListDynamic> for List<Value> {
    fn from(l: ListDynamic) -> Self {
        l.into_list()
    }
}

impl std::fmt::Display for ListDynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_list(&self.0)
    }
}

impl serde::Serialize for ListDynamic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_signed_value(serializer, list_ty!(self.1.clone()), &self.0)
    }
}

struct ListDynamicSeed(Option<Type>);

impl<'de> serde::de::DeserializeSeed<'de> for ListDynamicSeed {
    type Value = ListDynamic;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor(Option<Type>);
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = ListDynamic;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a list or varargs value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut list = List::new();
                while let Some(typed_value) = seq.next_element_seed(DynamicSeed(self.0.clone()))? {
                    // Drop the type information and transform into a simple value. The type
                    // information is already stored with the list.
                    list.push(typed_value.into_value());
                }
                Ok(ListDynamic(list, self.0))
            }
        }
        deserializer.deserialize_seq(Visitor(self.0))
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct MapDynamic {
    value: Map<Value, Value>,
    key_type: Option<Type>,
    value_type: Option<Type>,
}

impl MapDynamic {
    pub fn into_value(self) -> Value {
        Value::Map(self.value)
    }

    pub fn into_map(self) -> Map<Value, Value> {
        self.value
    }

    fn ty(&self) -> Type {
        Type::Map {
            key: self.key_type.clone().map(Box::new),
            value: self.value_type.clone().map(Box::new),
        }
    }
}

impl ty::DynamicGetType for MapDynamic {
    fn ty(&self) -> Option<Type> {
        Some(self.ty())
    }

    fn deep_ty(&self) -> Type {
        self.ty()
    }
}

impl From<MapDynamic> for Value {
    fn from(m: MapDynamic) -> Self {
        m.into_value()
    }
}

impl From<MapDynamic> for Map<Value, Value> {
    fn from(m: MapDynamic) -> Self {
        m.into_map()
    }
}

impl std::fmt::Display for MapDynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl serde::Serialize for MapDynamic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_signed_value(
            serializer,
            map_ty!(self.key_type.clone(), self.value_type.clone()),
            &self.value,
        )
    }
}

struct MapDynamicSeed {
    key: Option<Type>,
    value: Option<Type>,
}

impl<'de> serde::de::DeserializeSeed<'de> for MapDynamicSeed {
    type Value = MapDynamic;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor {
            key: Option<Type>,
            value: Option<Type>,
        }
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = MapDynamic;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut pair_vec = Vec::new();
                while let Some((key, value)) = map.next_entry_seed(
                    DynamicSeed(self.key.clone()),
                    DynamicSeed(self.value.clone()),
                )? {
                    // Drop the type information and transform into simple values. The types
                    // information are already stored with the map.
                    pair_vec.push((key.into_value(), value.into_value()));
                }
                Ok(MapDynamic {
                    value: Map::from_iter(pair_vec),
                    key_type: self.key,
                    value_type: self.value,
                })
            }
        }
        deserializer.deserialize_map(Visitor {
            key: self.key,
            value: self.value,
        })
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct TupleDynamic(Tuple, ty::TupleType);

impl TupleDynamic {
    pub fn into_value(self) -> Value {
        Value::Tuple(self.0)
    }

    pub fn into_tuple(self) -> Tuple {
        self.0
    }

    fn ty(&self) -> Type {
        Type::Tuple(self.1.clone())
    }
}

impl ty::DynamicGetType for TupleDynamic {
    fn ty(&self) -> Option<Type> {
        Some(self.ty())
    }

    fn deep_ty(&self) -> Type {
        self.ty()
    }
}

impl From<TupleDynamic> for Value {
    fn from(t: TupleDynamic) -> Self {
        t.into_value()
    }
}

impl From<TupleDynamic> for Tuple {
    fn from(t: TupleDynamic) -> Self {
        t.into_tuple()
    }
}

impl std::fmt::Display for TupleDynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl serde::Serialize for TupleDynamic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_signed_value(serializer, Type::Tuple(self.1.clone()), &self.0)
    }
}

struct TupleDynamicSeed(ty::TupleType);

impl<'de> serde::de::DeserializeSeed<'de> for TupleDynamicSeed {
    type Value = TupleDynamic;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn deser_tuple_from_seq<'de, A, I, E>(
            mut seq: A,
            element_types: I,
            expecting: &E,
        ) -> Result<Tuple, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
            I: IntoIterator<Item = Option<Type>>,
            E: serde::de::Expected,
        {
            let mut elements = Vec::new();
            for element_type in element_types {
                match seq.next_element_seed(DynamicSeed(element_type))? {
                    Some(element) => elements.push(element.into_value()),
                    None => {
                        return Err(serde::de::Error::invalid_length(elements.len(), expecting))
                    }
                };
            }
            Ok(Tuple::from_vec(elements))
        }

        struct TupleVisitor(ty::TupleType);
        impl<'de> serde::de::Visitor<'de> for TupleVisitor {
            type Value = TupleDynamic;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a tuple value of size {len}", len = self.0.len())
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let tuple = deser_tuple_from_seq(seq, self.0.element_types(), &self)?;
                Ok(TupleDynamic(tuple, self.0))
            }
        }

        struct StructVisitor(String, Vec<ty::StructField>);
        impl<'de> serde::de::Visitor<'de> for StructVisitor {
            type Value = TupleDynamic;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a struct value of size {len}", len = self.1.len(),)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let tuple = deser_tuple_from_seq(
                    seq,
                    self.1.iter().map(|field| field.value_type.clone()),
                    &self,
                )?;
                Ok(TupleDynamic(tuple, ty::TupleType::Struct(self.0, self.1)))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut elements = Vec::new();
                for field in &self.1 {
                    match map.next_entry_seed(
                        std::marker::PhantomData::<&str>,
                        DynamicSeed(field.value_type.clone()),
                    )? {
                        Some((key, value)) if key == field.name => {
                            elements.push(value.into_value())
                        }
                        Some(_) => {
                            return Err(serde::de::Error::custom("missing field `{field.name}`"))
                        }
                        None => {
                            return Err(serde::de::Error::invalid_length(elements.len(), &self))
                        }
                    };
                }

                let tuple = Tuple::from_vec(elements);
                Ok(TupleDynamic(tuple, ty::TupleType::Struct(self.0, self.1)))
            }
        }

        match self.0 {
            ty::TupleType::Tuple(_) | ty::TupleType::TupleStruct(_, _) => {
                deserializer.deserialize_tuple(self.0.len(), TupleVisitor(self.0))
            }
            ty::TupleType::Struct(name, fields) => {
                deserializer.deserialize_tuple(fields.len(), StructVisitor(name, fields))
            }
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, thiserror::Error)]
pub struct TypeMismatchError {
    expected: Option<Type>,
    actual: Option<Type>,
}

impl std::fmt::Display for TypeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("type mismatch error, expected ")?;
        match &self.expected {
            Some(t) => t.fmt(f)?,
            None => f.write_str("Dynamic")?,
        };
        f.write_str(", got ")?;
        match &self.actual {
            Some(t) => t.fmt(f),
            None => f.write_str("Dynamic"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{list_ty, map_ty, option_ty, struct_ty};
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_dynamic_serde() {
        let value_type = struct_ty! {
            MyStruct {
                an_int: Type::Int32,
                a_raw: Type::Raw,
                an_option: option_ty!(map_ty!(Type::String, list_ty!(Type::Bool))),
            }
        };
        let value = Value::Tuple(Tuple::from_vec(vec![
            Value::Number(Number::Int32(42)),
            Value::Raw(Raw::from(vec![1, 2, 3])),
            Value::Option(Box::new(Some(Value::Map(Map::from_iter(vec![
                (
                    Value::String(String::from("true_true")),
                    Value::List(vec![Value::Bool(true), Value::Bool(true)]),
                ),
                (
                    Value::String(String::from("false_true")),
                    Value::List(vec![Value::Bool(false), Value::Bool(true)]),
                ),
                (
                    Value::String(String::from("true_false")),
                    Value::List(vec![Value::Bool(true), Value::Bool(false)]),
                ),
                (
                    Value::String(String::from("false_false")),
                    Value::List(vec![Value::Bool(false), Value::Bool(false)]),
                ),
            ]))))),
        ]));
        let dynamic = Dynamic::new(value, Some(value_type)).unwrap();
        assert_tokens(
            &dynamic,
            &[
                Token::Tuple { len: 2 },
                Token::Str("(ir+{s[b]})<MyStruct,an_int,a_raw,an_option>"),
                Token::Tuple { len: 3 },
                Token::I32(42),
                Token::Bytes(&[1, 2, 3]),
                Token::Some,
                Token::Map { len: Some(4) },
                Token::Str("true_true"),
                Token::Seq { len: Some(2) },
                Token::Bool(true),
                Token::Bool(true),
                Token::SeqEnd,
                Token::Str("false_true"),
                Token::Seq { len: Some(2) },
                Token::Bool(false),
                Token::Bool(true),
                Token::SeqEnd,
                Token::Str("true_false"),
                Token::Seq { len: Some(2) },
                Token::Bool(true),
                Token::Bool(false),
                Token::SeqEnd,
                Token::Str("false_false"),
                Token::Seq { len: Some(2) },
                Token::Bool(false),
                Token::Bool(false),
                Token::SeqEnd,
                Token::MapEnd,
                Token::TupleEnd,
                Token::TupleEnd,
            ],
        );
    }
}
