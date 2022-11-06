use crate::{AnyValue, Signature, Type};
use indexmap::IndexMap;

impl serde::Serialize for AnyValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut serializer = serializer.serialize_tuple(2)?;
        let value_type = self.runtime_type();
        serializer.serialize_element(&Signature::from(value_type))?;
        match self {
            AnyValue::Void => serializer.serialize_element(&()),
            AnyValue::Bool(b) => serializer.serialize_element(b),
            AnyValue::Int8(i) => serializer.serialize_element(i),
            AnyValue::UInt8(u) => serializer.serialize_element(u),
            AnyValue::Int16(i) => serializer.serialize_element(i),
            AnyValue::UInt16(u) => serializer.serialize_element(u),
            AnyValue::Int32(i) => serializer.serialize_element(i),
            AnyValue::UInt32(u) => serializer.serialize_element(u),
            AnyValue::Int64(i) => serializer.serialize_element(i),
            AnyValue::UInt64(u) => serializer.serialize_element(u),
            AnyValue::Float(f) => serializer.serialize_element(f),
            AnyValue::Double(d) => serializer.serialize_element(d),
            AnyValue::String(s) => serializer.serialize_element(s),
            AnyValue::Raw(r) => serializer.serialize_element(serde_bytes::Bytes::new(r)),
            AnyValue::Option { option, .. } => serializer.serialize_element(option),
            AnyValue::List { list, .. } => serializer.serialize_element(list),
            AnyValue::Map { map, .. } => {
                struct AsMap<T>(T);
                impl serde::Serialize for AsMap<&Vec<(AnyValue, AnyValue)>> {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: serde::Serializer,
                    {
                        serializer.collect_map(self.0.iter().map(|(k, v)| (k, v)))
                    }
                }
                serializer.serialize_element(&AsMap(map))
            }
            AnyValue::Tuple(elements) | AnyValue::TupleStruct { elements, .. } => {
                struct AsTuple<T>(T);
                impl serde::Serialize for AsTuple<&Vec<AnyValue>> {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: serde::Serializer,
                    {
                        let mut serializer = serializer.serialize_tuple(self.0.len())?;
                        for element in self.0 {
                            serializer.serialize_element(element)?;
                        }
                        serializer.end()
                    }
                }
                serializer.serialize_element(&AsTuple(elements))
            }
            AnyValue::Struct { fields, .. } => {
                struct AsTuple<T>(T);
                impl serde::Serialize for AsTuple<&IndexMap<String, AnyValue>> {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: serde::Serializer,
                    {
                        let mut serializer = serializer.serialize_tuple(self.0.len())?;
                        for (_key, element) in self.0 {
                            serializer.serialize_element(element)?;
                        }
                        serializer.end()
                    }
                }
                serializer.serialize_element(&AsTuple(fields))
            }
        }?;
        serializer.end()
    }
}

pub fn to_any_value<T>(value: &T, value_type: &Type) -> Result<AnyValue, Error>
where
    T: serde::Serialize + ?Sized,
{
    value.serialize(Serializer::new(value_type))
}

struct Serializer<'t> {
    value_type: &'t Type,
}

impl<'t> Serializer<'t> {
    fn new(value_type: &'t Type) -> Self {
        Self { value_type }
    }

    fn check_value_type(&self, t: Type) -> Result<(), Error> {
        match self.value_type != &t {
            false => Err(Error::UnexpectedValueType {
                expected: self.value_type.clone(),
                actual: t.to_string(),
            }),
            true => Ok(()),
        }
    }
}

impl<'t> serde::Serializer for Serializer<'t> {
    type Ok = AnyValue;
    type Error = Error;

    type SerializeSeq = ListSerializer;
    type SerializeTuple = TupleSerializer<std::slice::Iter<'t, Type>>;
    type SerializeTupleStruct = TupleStructSerializer<std::slice::Iter<'t, Type>>;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer<indexmap::map::Iter<'t, String, Type>>;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Bool)?;
        Ok(AnyValue::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Int8)?;
        Ok(AnyValue::Int8(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Int16)?;
        Ok(AnyValue::Int16(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Int32)?;
        Ok(AnyValue::Int32(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Int64)?;
        Ok(AnyValue::Int64(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::UInt8)?;
        Ok(AnyValue::UInt8(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::UInt16)?;
        Ok(AnyValue::UInt16(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::UInt32)?;
        Ok(AnyValue::UInt32(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::UInt64)?;
        Ok(AnyValue::UInt64(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Float)?;
        Ok(AnyValue::Float(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Double)?;
        Ok(AnyValue::Double(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::String)?;
        let mut buf = [0; 4];
        let s = v.encode_utf8(&mut buf);
        self.serialize_str(s)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::String)?;
        Ok(AnyValue::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Raw)?;
        // OPTIMIZE: Do not copy bytes, but reference them
        Ok(AnyValue::Raw(v.into()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        match self.value_type {
            Type::Option(value_type) => Ok(AnyValue::Option {
                value_type: value_type.as_ref().clone(),
                option: None,
            }),
            _ => Err(Error::UnexpectedValueType {
                expected: self.value_type.clone(),
                actual: "an option type".into(),
            }),
        }
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value_type = match self.value_type {
            Type::Option(value_type) => value_type.as_ref(),
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: "an option type".into(),
                });
            }
        };
        Ok(AnyValue::Option {
            value_type: value_type.clone(),
            option: Some({
                let value = to_any_value(value, value_type)?;
                Box::new(value)
            }),
        })
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.check_value_type(Type::Void)?;
        Ok(AnyValue::Void)
    }

    fn serialize_unit_struct(self, struct_name: &'static str) -> Result<Self::Ok, Self::Error> {
        if !matches!(self.value_type, Type::TupleStruct { name, elements } if name == struct_name && elements.is_empty())
        {
            return Err(Error::UnexpectedValueType {
                expected: self.value_type.clone(),
                actual: format!("a unit struct type named {struct_name}"),
            });
        }
        Ok(AnyValue::TupleStruct {
            name: struct_name.into(),
            elements: vec![],
        })
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
        struct_name: &'static str,
        element: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let element_type = match self.value_type {
            Type::TupleStruct { name, elements } if name == struct_name && elements.len() == 1 => {
                elements.get(0).unwrap()
            }
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a newtype struct type named {struct_name}"),
                });
            }
        };
        let value = to_any_value(element, element_type)?;
        Ok(AnyValue::TupleStruct {
            name: struct_name.into(),
            elements: vec![value],
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
        todo!("enums are not yet supported as values")
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let value_type = match self.value_type {
            Type::List(value_type) => value_type.as_ref().clone(),
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: "a list type".into(),
                });
            }
        };
        Ok(ListSerializer::new(value_type))
    }

    fn serialize_tuple(self, tuple_len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let element_types = match self.value_type {
            Type::Tuple(element_types) if element_types.len() == tuple_len => element_types,
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a tuple type of size {tuple_len}"),
                });
            }
        };
        Ok(TupleSerializer::new(element_types))
    }

    fn serialize_tuple_struct(
        self,
        struct_name: &'static str,
        struct_len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        let element_types = match self.value_type {
            Type::TupleStruct { name, elements }
                if name == struct_name && elements.len() == struct_len =>
            {
                elements
            }
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!(
                        "a tuple struct type of name {struct_name} of size {struct_len}"
                    ),
                });
            }
        };
        Ok(TupleStructSerializer::new(struct_name, element_types))
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

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let (key_type, value_type) = match self.value_type {
            Type::Map { key, value } => (key.as_ref().clone(), value.as_ref().clone()),
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: "a map type".into(),
                });
            }
        };
        Ok(MapSerializer::new(key_type, value_type))
    }

    fn serialize_struct(
        self,
        struct_name: &'static str,
        struct_len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let field_types = match self.value_type {
            Type::Struct { name, fields } if name == struct_name && fields.len() == struct_len => {
                fields
            }
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a struct type of size {struct_len}"),
                });
            }
        };
        Ok(StructSerializer::new(struct_name, field_types.iter()))
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

pub struct ListSerializer {
    value_type: Type,
    elements: Vec<AnyValue>,
}

impl ListSerializer {
    fn new(value_type: Type) -> Self {
        Self {
            value_type,
            elements: Vec::new(),
        }
    }
}

impl serde::ser::SerializeSeq for ListSerializer {
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = to_any_value(value, &self.value_type)?;
        self.elements.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::List {
            value_type: self.value_type,
            list: self.elements,
        })
    }
}

pub struct MapSerializer {
    elements: Vec<(AnyValue, AnyValue)>,
    value_type: Type,
    key: Option<AnyValue>,
    key_type: Type,
}

impl MapSerializer {
    fn new(key_type: Type, value_type: Type) -> Self {
        Self {
            elements: Vec::new(),
            key_type,
            key: None,
            value_type,
        }
    }
}

impl serde::ser::SerializeMap for MapSerializer {
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = to_any_value(key, &self.key_type)?;
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
        let value = to_any_value(value, &self.value_type)?;
        self.elements.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::Map {
            key_type: self.key_type,
            value_type: self.value_type,
            map: self.elements,
        })
    }
}

pub struct TupleSerializer<I> {
    elements: Vec<AnyValue>,
    element_types: I,
}

impl<I> TupleSerializer<I>
where
    I: Iterator,
{
    fn new<E>(element_types: E) -> Self
    where
        E: IntoIterator<IntoIter = I>,
    {
        Self {
            elements: Vec::new(),
            element_types: element_types.into_iter(),
        }
    }
}

fn serialize_tuple_element<'t, T, I>(value: &T, types: &mut I) -> Result<AnyValue, Error>
where
    T: serde::Serialize + ?Sized,
    I: Iterator<Item = &'t Type>,
{
    match types.next() {
        Some(t) => to_any_value(value, t),
        None => unreachable!("the tuple size precondition is not verified"),
    }
}

impl<'t, I> serde::ser::SerializeTuple for TupleSerializer<I>
where
    I: Iterator<Item = &'t Type>,
{
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let element = serialize_tuple_element(value, &mut self.element_types)?;
        self.elements.push(element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::Tuple(self.elements))
    }
}

pub struct TupleStructSerializer<I> {
    name: String,
    elements: Vec<AnyValue>,
    element_types: I,
}

impl<I> TupleStructSerializer<I>
where
    I: Iterator,
{
    fn new<S, E>(name: S, element_types: E) -> Self
    where
        S: Into<String>,
        E: IntoIterator<IntoIter = I>,
    {
        Self {
            name: name.into(),
            elements: Vec::new(),
            element_types: element_types.into_iter(),
        }
    }
}

impl<'t, I> serde::ser::SerializeTupleStruct for TupleStructSerializer<I>
where
    I: Iterator<Item = &'t Type>,
{
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let element = serialize_tuple_element(value, &mut self.element_types)?;
        self.elements.push(element);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::TupleStruct {
            name: self.name,
            elements: self.elements,
        })
    }
}

struct StructSerializer<I> {
    name: String,
    fields: IndexMap<String, AnyValue>,
    field_types: I,
}

impl<I> StructSerializer<I>
where
    I: Iterator,
{
    fn new<S, F>(name: S, field_types: F) -> Self
    where
        S: Into<String>,
        F: IntoIterator<IntoIter = I>,
    {
        Self {
            name: name.into(),
            fields: IndexMap::new(),
            field_types: field_types.into_iter(),
        }
    }
}

impl<'t, I> serde::ser::SerializeStruct for StructSerializer<I>
where
    I: Iterator<Item = (&'t String, &'t Type)> + Clone,
{
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let field_type = match self.field_types.clone().find(|(k, _)| *k == key) {
            Some((_field_name, field_type)) => field_type,
            None => {
                return Err(Error::UnexpectedTupleField(
                    key.into(),
                    self.field_types
                        .clone()
                        .map(|(k, _)| format!(",{k}"))
                        .collect(),
                ))
            }
        };
        let value = to_any_value(value, field_type)?;
        self.fields.insert(key.into(), value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::Struct {
            name: self.name,
            fields: self.fields,
        })
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("unexpected value type, expected {expected} but got {actual}")]
    UnexpectedValueType { expected: Type, actual: String },

    #[error("unexpected tuple field \"{0}\", expected any of {1}")]
    UnexpectedTupleField(String, String),

    #[error("error: {0}")]
    Custom(String),
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}
