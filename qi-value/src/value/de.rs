use super::Value;
use crate::{
    dynamic::{self},
    ty::{self, DisplayTypeOption, DisplayTypeTuple},
    IntoValue, Map, Object, Type,
};
use serde::de::DeserializeSeed;
use std::marker::PhantomData;

impl<'de: 'a, 'a> serde::Deserialize<'de> for Value<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(AnyVisitor::new())
    }
}

pub fn deserialize_value_of_type<'de: 'a, 'a, 't, D>(
    deserializer: D,
    value_type: Option<&'t Type>,
) -> Result<Value<'a>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    ValueTypeSeed::new(value_type).deserialize(deserializer)
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ValueTypeSeed<'a, 't> {
    value_type: Option<&'t Type>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, 't> ValueTypeSeed<'a, 't> {
    pub fn new(value_type: Option<&'t Type>) -> Self {
        Self {
            value_type,
            phantom: PhantomData,
        }
    }
}

impl<'de: 'a, 'a, 't> serde::de::DeserializeSeed<'de> for ValueTypeSeed<'a, 't> {
    type Value = Value<'a>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn deserialize_tuple<'de: 'a, 'a, 't, D, I>(
            deserializer: D,
            len: usize,
            iter: I,
        ) -> Result<Value<'a>, D::Error>
        where
            D: serde::Deserializer<'de>,
            I: IntoIterator<Item = Option<&'t Type>>,
        {
            let tuple =
                deserializer.deserialize_tuple(len, TupleVisitor::new(Vec::from_iter(iter)))?;
            Ok(Value::Tuple(tuple))
        }
        use serde::Deserialize;
        match self.value_type {
            Some(Type::Unit) => deserializer.deserialize_unit(AnyVisitor::new()),
            Some(Type::Bool) => deserializer.deserialize_bool(AnyVisitor::new()),
            Some(Type::Int8) => deserializer.deserialize_i8(AnyVisitor::new()),
            Some(Type::UInt8) => deserializer.deserialize_u8(AnyVisitor::new()),
            Some(Type::Int16) => deserializer.deserialize_i16(AnyVisitor::new()),
            Some(Type::UInt16) => deserializer.deserialize_u16(AnyVisitor::new()),
            Some(Type::Int32) => deserializer.deserialize_i32(AnyVisitor::new()),
            Some(Type::UInt32) => deserializer.deserialize_u32(AnyVisitor::new()),
            Some(Type::Int64) => deserializer.deserialize_i64(AnyVisitor::new()),
            Some(Type::UInt64) => deserializer.deserialize_u64(AnyVisitor::new()),
            Some(Type::Float32) => deserializer.deserialize_f32(AnyVisitor::new()),
            Some(Type::Float64) => deserializer.deserialize_f64(AnyVisitor::new()),
            Some(Type::String) => deserializer.deserialize_str(AnyVisitor::new()),
            Some(Type::Raw) => deserializer.deserialize_bytes(AnyVisitor::new()),
            Some(Type::Object) => Ok(Object::deserialize(deserializer)?.into_value()),
            Some(Type::Option(value)) => {
                let opt = deserializer.deserialize_option(OptionVisitor::new(value.as_deref()))?;
                Ok(Value::Option(opt.map(Box::new)))
            }
            Some(Type::List(value) | Type::VarArgs(value)) => {
                let list = deserializer.deserialize_seq(ListVisitor::new(value.as_deref()))?;
                Ok(Value::List(list))
            }
            Some(Type::Map { key, value }) => {
                let map = deserializer
                    .deserialize_map(MapVisitor::new(key.as_deref(), value.as_deref()))?;
                Ok(Value::Map(map))
            }
            Some(Type::Tuple(
                ty::Tuple::Tuple(elements) | ty::Tuple::TupleStruct { elements, .. },
            )) => deserialize_tuple(
                deserializer,
                elements.len(),
                elements.iter().map(Option::as_ref),
            ),
            Some(Type::Tuple(ty::Tuple::Struct { fields, .. })) => deserialize_tuple(
                deserializer,
                fields.len(),
                fields.iter().map(|field| field.ty.as_ref()),
            ),
            None => {
                let value = dynamic::deserialize(deserializer)?;
                Ok(Value::Dynamic(Box::new(value)))
            }
        }
    }
}

struct AnyVisitor<'a> {
    phantom: PhantomData<&'a ()>,
}

impl<'a> AnyVisitor<'a> {
    fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<'de: 'a, 'a> serde::de::Visitor<'de> for AnyVisitor<'a> {
    type Value = Value<'a>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_str(v.encode_utf8(&mut [0u8; 4]))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.into())
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into_value())
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_byte_buf(v.into())
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Raw(v.into()))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Raw(v.into()))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Option(None))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde::Deserialize::deserialize(deserializer)?;
        Ok(Value::Option(Some(Box::new(value))))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Unit)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde::Deserialize::deserialize(deserializer)?;
        Ok(Value::Tuple(vec![value]))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut values = seq.size_hint().map(Vec::with_capacity).unwrap_or_default();
        while let Some(value) = seq.next_element()? {
            values.push(value);
        }
        Ok(Value::List(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut values = map.size_hint().map(Map::with_capacity).unwrap_or_default();
        while let Some((key, value)) = map.next_entry()? {
            values.insert(key, value);
        }
        Ok(Value::Map(values))
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        use serde::de::VariantAccess;
        let (index, variant): (u32, _) = data.variant()?;
        let value = variant.newtype_variant()?;
        Ok(Value::Tuple(vec![index.into_value(), value]))
    }
}

struct OptionVisitor<'a, 't> {
    value_type: Option<&'t Type>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, 't> OptionVisitor<'a, 't> {
    fn new(value_type: Option<&'t Type>) -> Self {
        Self {
            value_type,
            phantom: PhantomData,
        }
    }
}

impl<'de: 'a, 'a, 't> serde::de::Visitor<'de> for OptionVisitor<'a, 't> {
    type Value = Option<Value<'a>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "an option of {}",
            DisplayTypeOption(&self.value_type)
        )
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = ValueTypeSeed::new(self.value_type).deserialize(deserializer)?;
        Ok(Some(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(None)
    }
}

struct ListVisitor<'a, 't> {
    value_type: Option<&'t Type>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, 't> ListVisitor<'a, 't> {
    fn new(value_type: Option<&'t Type>) -> Self {
        Self {
            value_type,
            phantom: PhantomData,
        }
    }
}

impl<'de: 'a, 'a, 't> serde::de::Visitor<'de> for ListVisitor<'a, 't> {
    type Value = Vec<Value<'a>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a list of {}",
            DisplayTypeOption(&self.value_type)
        )
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut values = seq.size_hint().map(Vec::with_capacity).unwrap_or_default();
        while let Some(value) = seq.next_element_seed(ValueTypeSeed::new(self.value_type))? {
            values.push(value);
        }
        Ok(values)
    }
}

struct MapVisitor<'a, 't> {
    key_type: Option<&'t Type>,
    value_type: Option<&'t Type>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, 't> MapVisitor<'a, 't> {
    fn new(key_type: Option<&'t Type>, value_type: Option<&'t Type>) -> Self {
        Self {
            key_type,
            value_type,
            phantom: PhantomData,
        }
    }
}

impl<'de: 'a, 'a, 't> serde::de::Visitor<'de> for MapVisitor<'a, 't> {
    type Value = Map<Value<'a>, Value<'a>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a map of {} to {}",
            DisplayTypeOption(&self.key_type),
            DisplayTypeOption(&self.value_type)
        )
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut values = map.size_hint().map(Map::with_capacity).unwrap_or_default();
        while let Some((key, value)) = map.next_entry_seed(
            ValueTypeSeed::new(self.key_type),
            ValueTypeSeed::new(self.value_type),
        )? {
            values.insert(key, value);
        }
        Ok(values)
    }
}

struct TupleVisitor<'a, 't> {
    element_types: Vec<Option<&'t Type>>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, 't> TupleVisitor<'a, 't> {
    fn new(element_types: Vec<Option<&'t Type>>) -> Self {
        Self {
            element_types,
            phantom: PhantomData,
        }
    }
}

impl<'de: 'a, 'a, 't> serde::de::Visitor<'de> for TupleVisitor<'a, 't> {
    type Value = Vec<Value<'a>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a tuple of {}",
            DisplayTypeTuple(&self.element_types)
        )
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        use serde::de::Error;
        let expected_len = self.element_types.len();
        let mut values = Vec::with_capacity(expected_len);
        let mut element_types_iter = self.element_types.iter().enumerate();
        loop {
            let (index, element_type) = match element_types_iter.next() {
                Some(element_type) => element_type,
                None => break Ok(values),
            };
            let value = seq
                .next_element_seed(ValueTypeSeed::new(*element_type))?
                .ok_or_else(|| A::Error::invalid_length(index, &self))?;
            values.push(value);
        }
    }
}
