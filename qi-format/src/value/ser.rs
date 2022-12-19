use super::*;

pub fn to_value<T>(value: &T) -> Value<'static>
where
    T: serde::Serialize + ?Sized,
{
    value
        .serialize(Serializer)
        .expect("logic error: serializing into a value is infallible")
}

struct Serializer;

impl serde::Serializer for Serializer {
    type Ok = Value<'static>;
    type Error = Infallible;

    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeTuple;
    type SerializeTupleStruct = SerializeTuple;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeTuple;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Number::from(v)))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(String::from(v.to_owned())))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Raw::from(v.to_owned())))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(None))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value = value.serialize(self)?;
        Ok(Value::from(Some(value)))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::unit())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Tuple::new(vec![])))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!("enums are not yet supported as values")
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        element: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value = element.serialize(self)?;
        Ok(Value::from(Tuple::new(vec![value])))
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
        todo!("enums are not yet supported as values")
    }

    fn serialize_seq(
        self,
        _len: std::option::Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeSeq::new())
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SerializeTuple::new(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SerializeTuple::new(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!("enums are not yet supported as values")
    }

    fn serialize_map(
        self,
        _len: std::option::Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap::new())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeTuple::new(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!("enums are not yet supported as values")
    }
}

pub struct SerializeSeq {
    list: List<'static>,
}

impl SerializeSeq {
    fn new() -> Self {
        Self { list: List::new() }
    }
}

impl serde::ser::SerializeSeq for SerializeSeq {
    type Ok = Value<'static>;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = value.serialize(Serializer)?;
        self.list.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::List(self.list))
    }
}

pub struct SerializeMap {
    map: Map<'static>,
    key: std::option::Option<Value<'static>>,
}

impl SerializeMap {
    fn new() -> Self {
        Self {
            map: Map::default(),
            key: None,
        }
    }
}

impl serde::ser::SerializeMap for SerializeMap {
    type Ok = Value<'static>;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = key.serialize(Serializer)?;
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
            .expect("logic error: missing key before value");
        let value = value.serialize(Serializer)?;
        self.map.0.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

pub struct SerializeTuple {
    elements: Vec<Value<'static>>,
}

impl SerializeTuple {
    fn new(len: usize) -> Self {
        Self {
            elements: Vec::with_capacity(len),
        }
    }
}

impl serde::ser::SerializeTuple for SerializeTuple {
    type Ok = Value<'static>;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let element = value.serialize(Serializer)?;
        self.elements.push(element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Tuple::new(self.elements)))
    }
}

impl serde::ser::SerializeTupleStruct for SerializeTuple {
    type Ok = Value<'static>;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let element = value.serialize(Serializer)?;
        self.elements.push(element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Tuple::new(self.elements)))
    }
}

impl serde::ser::SerializeStruct for SerializeTuple {
    type Ok = Value<'static>;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let element = value.serialize(Serializer)?;
        self.elements.push(element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(Tuple::new(self.elements)))
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Infallible {}

impl serde::ser::Error for Infallible {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        unreachable!("infaillible serialization")
    }
}

pub type Error = Infallible;

impl<'v> Serialize for Value<'v> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Bool(b) => b.serialize(serializer),
            Value::Number(n) => n.serialize(serializer),
            Value::String(s) => s.serialize(serializer),
            Value::Raw(r) => r.serialize(serializer),
            Value::Option(o) => o.serialize(serializer),
            Value::List(l) => l.serialize(serializer),
            Value::Map(m) => m.serialize(serializer),
            Value::Tuple(tuple) => tuple.serialize(serializer),
        }
    }
}

impl<'v> Serialize for Map<'v> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut serializer = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            serializer.serialize_entry(key, value)?;
        }
        serializer.end()
    }
}

impl<'v> Serialize for AnnotatedValue<'v> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}
