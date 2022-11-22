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
        Ok(Value::Int8(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int16(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int32(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int64(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::UnsignedInt8(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::UnsignedInt16(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::UnsignedInt32(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::UnsignedInt64(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float32(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float64(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        let s = v.encode_utf8(&mut buf);
        self.serialize_str(s)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(v.to_owned().into()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Raw(v.to_owned().into()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Option(None))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value = to_value(value);
        let value = Box::new(value);
        Ok(Value::Option(Some(value)))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Unit)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Tuple(Tuple { elements: vec![] }))
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
        let value = to_value(element);
        Ok(Value::Tuple(Tuple {
            elements: vec![value],
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
        todo!("enums are not yet supported as values")
    }

    fn serialize_seq(self, _len: StdOption<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeSeq::new())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SerializeTuple::new())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SerializeTuple::new())
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

    fn serialize_map(self, _len: StdOption<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap::new())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeTuple::new())
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
        let value = to_value(value);
        self.list.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::List(self.list))
    }
}

pub struct SerializeMap {
    map: Map<'static>,
    key: StdOption<Value<'static>>,
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
        let key = to_value(key);
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
        let value = to_value(value);
        self.map.0.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

pub struct SerializeTuple {
    tuple: Tuple<'static>,
}

impl SerializeTuple {
    fn new() -> Self {
        Self {
            tuple: Tuple::default(),
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
        let element = to_value(value);
        self.tuple.elements.push(element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Tuple(self.tuple))
    }
}

impl serde::ser::SerializeTupleStruct for SerializeTuple {
    type Ok = Value<'static>;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let element = to_value(value);
        self.tuple.elements.push(element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Tuple(self.tuple))
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
        let element = to_value(value);
        self.tuple.elements.push(element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Tuple(self.tuple))
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
            Value::Unit => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Int8(i) => serializer.serialize_i8(*i),
            Value::UnsignedInt8(u) => serializer.serialize_u8(*u),
            Value::Int16(i) => serializer.serialize_i16(*i),
            Value::UnsignedInt16(u) => serializer.serialize_u16(*u),
            Value::Int32(i) => serializer.serialize_i32(*i),
            Value::UnsignedInt32(u) => serializer.serialize_u32(*u),
            Value::Int64(i) => serializer.serialize_i64(*i),
            Value::UnsignedInt64(u) => serializer.serialize_u64(*u),
            Value::Float32(f) => serializer.serialize_f32(*f),
            Value::Float64(d) => serializer.serialize_f64(*d),
            Value::String(s) => serializer.serialize_str(s),
            Value::Raw(r) => serializer.serialize_bytes(r),
            Value::Option(o) => match o {
                Some(v) => serializer.serialize_some(v),
                None => serializer.serialize_none(),
            },
            Value::List(l) => serializer.collect_seq(l.iter()),
            Value::Map(m) => serializer.collect_map(m.into_iter().map(|(k, v)| (k, v))),
            Value::Tuple(Tuple { elements }) => {
                use serde::ser::SerializeTuple;
                let mut serializer = serializer.serialize_tuple(elements.len())?;
                for elem in elements {
                    serializer.serialize_element(elem)?;
                }
                serializer.end()
            }
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

impl<'v> Serialize for Tuple<'v> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut serializer = serializer.serialize_tuple(self.elements.len())?;
        for element in &self.elements {
            serializer.serialize_element(element)?;
        }
        serializer.end()
    }
}
