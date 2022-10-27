use super::{tuple, Dynamic, Tuple};

pub fn to_dynamic<T>(value: &T) -> Result<Dynamic, Error>
where
    T: serde::Serialize + ?Sized,
{
    value.serialize(Serializer)
}

pub struct Serializer;

impl serde::Serializer for Serializer {
    type Ok = Dynamic;
    type Error = Error;

    type SerializeSeq = ListSerializer;
    type SerializeTuple = TupleSerializer<Dynamic>;
    type SerializeTupleStruct = TupleSerializer<Dynamic>;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = MapSerializer;
    type SerializeStruct = TupleSerializer<tuple::Field>;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Int8(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Int16(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Int32(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Int64(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::UInt8(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::UInt16(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::UInt32(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::UInt64(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Float(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Double(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        let s = v.encode_utf8(&mut buf);
        self.serialize_str(s)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        // OPTIMIZE: Do not copy bytes, but reference them
        Ok(Dynamic::Raw(v.into()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Optional(None))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value = to_dynamic(value)?;
        Ok(Dynamic::Optional(Some(Box::new(value))))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Void)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Tuple(Tuple {
            name: Some(name.to_string()),
            elements: tuple::Elements::Fields(vec![]),
        }))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnionAreNotSupported)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value = to_dynamic(value)?;
        Ok(Dynamic::Tuple(Tuple {
            name: Some(name.to_string()),
            elements: tuple::Elements::Raw(vec![value]),
        }))
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Err(Error::UnionAreNotSupported)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ListSerializer::new())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSerializer::new(None))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleSerializer::new(Some(name.to_string())))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::UnionAreNotSupported)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new())
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(TupleSerializer::new(Some(name.to_string())))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::UnionAreNotSupported)
    }
}

pub struct ListSerializer {
    elements: Vec<Dynamic>,
}

impl ListSerializer {
    fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }
}

impl serde::ser::SerializeSeq for ListSerializer {
    type Ok = Dynamic;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = to_dynamic(value)?;
        self.elements.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::List(self.elements))
    }
}

pub struct MapSerializer {
    elements: Vec<(Dynamic, Dynamic)>,
    key: Option<Dynamic>,
}

impl MapSerializer {
    fn new() -> Self {
        Self {
            elements: Vec::new(),
            key: None,
        }
    }
}

impl serde::ser::SerializeMap for MapSerializer {
    type Ok = Dynamic;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = to_dynamic(key)?;
        self.key = Some(key);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::Custom("key was not serialized".into()))?;
        let value = to_dynamic(value)?;
        self.elements.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Dynamic::Map(self.elements))
    }
}

pub struct TupleSerializer<T> {
    name: Option<String>,
    fields: Vec<T>,
}

impl<T> TupleSerializer<T> {
    fn new(name: Option<String>) -> Self {
        Self {
            name,
            fields: Vec::new(),
        }
    }

    fn into_value(self) -> Dynamic
    where
        tuple::Elements: FromIterator<T>,
    {
        Dynamic::Tuple(Tuple {
            name: self.name,
            elements: tuple::Elements::from_iter(self.fields),
        })
    }
}

impl TupleSerializer<Dynamic> {
    fn add_member<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let value = to_dynamic(value)?;
        self.fields.push(value);
        Ok(())
    }
}

impl TupleSerializer<tuple::Field> {
    fn add_field<T: ?Sized>(&mut self, name: &str, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let name = name.to_string();
        let dynamic = to_dynamic(value)?;
        self.fields.push(tuple::Field {
            name,
            element: dynamic,
        });
        Ok(())
    }
}

impl serde::ser::SerializeTuple for TupleSerializer<Dynamic> {
    type Ok = Dynamic;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.add_member(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

impl serde::ser::SerializeTupleStruct for TupleSerializer<Dynamic> {
    type Ok = Dynamic;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.add_member(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

impl serde::ser::SerializeStruct for TupleSerializer<tuple::Field> {
    type Ok = Dynamic;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.add_field(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {
    #[error("union types are not supported in the qi type system")]
    UnionAreNotSupported,

    #[error("unknown value type")]
    UnknownValueType,

    #[error("error: {0}")]
    Custom(String),
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl serde::Serialize for Dynamic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // TODO: Serialize as an enumeration.
        match self {
            Dynamic::Void => serializer.serialize_unit(),
            Dynamic::Bool(b) => b.serialize(serializer),
            Dynamic::Int8(i) => i.serialize(serializer),
            Dynamic::UInt8(i) => i.serialize(serializer),
            Dynamic::Int16(i) => i.serialize(serializer),
            Dynamic::UInt16(i) => i.serialize(serializer),
            Dynamic::Int32(i) => i.serialize(serializer),
            Dynamic::UInt32(i) => i.serialize(serializer),
            Dynamic::Int64(i) => i.serialize(serializer),
            Dynamic::UInt64(i) => i.serialize(serializer),
            Dynamic::Float(f) => f.serialize(serializer),
            Dynamic::Double(d) => d.serialize(serializer),
            Dynamic::String(s) => s.serialize(serializer),
            Dynamic::List(l) => l.serialize(serializer),
            Dynamic::Map(m) => {
                // Do not serialize the vector of pair directly, serialize it as a map instead.
                let mut map = serializer.serialize_map(Some(m.len()))?;
                use serde::ser::SerializeMap;
                for (key, value) in m {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Dynamic::Tuple(t) => t.serialize(serializer),
            Dynamic::Raw(r) => {
                // Do not serializ the vector of bytes directly, serialize it as bytes instead.
                serializer.serialize_bytes(r)
            }
            Dynamic::Optional(o) => o.serialize(serializer),
        }
    }
}