use super::{to_value, Error, TupleMember, Value};

pub struct Serializer;

impl serde::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = ListSerializer;
    type SerializeTuple = TupleSerializer;
    type SerializeTupleStruct = TupleSerializer;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = MapSerializer;
    type SerializeStruct = TupleSerializer;
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
        Ok(Value::UInt8(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::UInt16(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::UInt32(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::UInt64(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Double(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        let s = v.encode_utf8(&mut buf);
        self.serialize_str(s)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        // OPTIMIZE: Do not copy bytes, but reference them
        Ok(Value::Raw(v.into()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Optional(None))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value = to_value(value)?;
        Ok(Value::Optional(Some(Box::new(value))))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Void)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Tuple {
            name: Some(name.to_string()),
            members: vec![],
        })
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
        let value = to_value(value)?;
        Ok(Value::Tuple {
            name: Some(name.to_string()),
            members: vec![TupleMember { name: None, value }],
        })
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
    elements: Vec<Value>,
}

impl ListSerializer {
    fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }
}

impl serde::ser::SerializeSeq for ListSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = to_value(value)?;
        self.elements.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::List(self.elements))
    }
}

pub struct MapSerializer {
    elements: Vec<(Value, Value)>,
    key: Option<Value>,
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
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = to_value(key)?;
        self.key = Some(key);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = self.key.take().ok_or(Error::MissingMapKey)?;
        let value = to_value(value)?;
        self.elements.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.elements))
    }
}

pub struct TupleSerializer {
    name: Option<String>,
    members: Vec<TupleMember>,
}

impl TupleSerializer {
    fn new(name: Option<String>) -> Self {
        Self {
            name,
            members: Vec::new(),
        }
    }

    fn add_member<T: ?Sized>(&mut self, name: Option<String>, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let value = to_value(value)?;
        self.members.push(TupleMember { name, value });
        Ok(())
    }

    fn into_value(self) -> Value {
        Value::Tuple {
            name: self.name,
            members: self.members,
        }
    }
}

impl serde::ser::SerializeTuple for TupleSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.add_member(None, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

impl serde::ser::SerializeTupleStruct for TupleSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.add_member(None, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

impl serde::ser::SerializeStruct for TupleSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.add_member(Some(key.to_string()), value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn map_to_value() {
        let mut map = BTreeMap::new();
        map.insert(1, "hello".to_string());
        map.insert(2, "world".to_string());
        let value = to_value(&map).expect("serialization error");
        let expected = Value::Map(vec![
            (Value::Int32(1), Value::String("hello".to_string())),
            (Value::Int32(2), Value::String("world".to_string())),
        ]);
        assert_eq!(value, expected);
    }

    #[test]
    fn struct_to_value() {
        #[derive(serde::Serialize)]
        struct S {
            a: i32,
            b: f32,
            c: bool,
            d: Vec<&'static str>,
        };
        let s = S {
            a: 320,
            b: 1293.32,
            c: false,
            d: vec!["cookies", "muffins"],
        };
        let value = to_value(&s).expect("serialization error");
        let expected = Value::Tuple {
            name: Some("S".to_string()),
            members: vec![
                TupleMember {
                    name: Some("a".to_string()),
                    value: Value::Int32(320),
                },
                TupleMember {
                    name: Some("b".to_string()),
                    value: Value::Float(1293.32),
                },
                TupleMember {
                    name: Some("c".to_string()),
                    value: Value::Bool(false),
                },
                TupleMember {
                    name: Some("d".to_string()),
                    value: Value::List(vec![
                        Value::String("cookies".to_string()),
                        Value::String("muffins".to_string()),
                    ]),
                },
            ],
        };
        assert_eq!(value, expected);
    }
}
