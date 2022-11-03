use super::{tuple as value_tuple, AnyValue};
use crate::typesystem::r#type::{Type, tuple as type_tuple};

impl serde::Serialize for AnyValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        todo!()
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

    type SerializeSeq = ListSerializer<'t>;
    type SerializeTuple = TupleSerializer<AnyValue, std::slice::Iter<'t, Type>>;
    type SerializeTupleStruct = TupleSerializer<AnyValue, std::slice::Iter<'t, Type>>;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = MapSerializer<'t>;
    type SerializeStruct = TupleSerializer<value_tuple::Field, std::slice::Iter<'t, type_tuple::Field>>;
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
                value_type: **value_type,
                option: None,
            }),
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: "an option type".into(),
                });
            }
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

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        if !matches!(self.value_type, 
                     Type::Tuple(type_tuple::Tuple {
                        name: Some(tuple_name),
                        elements: type_tuple::Elements::Raw(v),
                     }) if tuple_name == name && v.is_empty())
        {
            return Err(Error::UnexpectedValueType {
                expected: self.value_type.clone(),
                actual: format!("a unit struct type named {name}"),
            });
        }
        Ok(AnyValue::Tuple(value_tuple::Tuple {
            name: Some(name.to_string()),
            elements: value_tuple::Elements::Raw(vec![]),
        }))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!("enums are not yet supported as an AnyValue")
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        element: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let element_type = match self.value_type {
            Type::Tuple(type_tuple::Tuple {
                name: Some(tuple_name),
                elements: type_tuple::Elements::Raw(v),
            }) if tuple_name == name && v.len() == 1 => v.get(0).unwrap(),
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a newtype struct type named {name}"),
                });
            }
        };
        let value = to_any_value(element, element_type)?;
        Ok(AnyValue::Tuple(value_tuple::Tuple {
            name: Some(name.to_string()),
            elements: value_tuple::Elements::Raw(vec![value]),
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
        todo!("enums are not yet supported as dynamic values")
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let value_type = match self.value_type {
            Type::List(value_type) => value_type.as_ref(),
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a list type"),
                });
            }
        };
        Ok(ListSerializer::new(value_type))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let element_types = match self.value_type {
            Type::Tuple(type_tuple::Tuple { name: None, elements: type_tuple::Elements::Raw(element_types) }) if element_types.len() == len => element_types,
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a tuple type of size {len}"),
                });
            },
        };
        Ok(TupleSerializer::new(None, element_types))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        let element_types = match self.value_type {
            Type::Tuple(type_tuple::Tuple { name: Some(tuple_name), elements: type_tuple::Elements::Raw(element_types) }) if element_types.len() == len => element_types,
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a tuple struct type of name {name} of size {len}"),
                });
            },
        };
        Ok(TupleSerializer::new(Some(name.to_string()), element_types))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!("enums are not yet supported as dynamic values")
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let (key_type, value_type) = match self.value_type {
            Type::Map { key, value } => (key, value),
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a map type"),
                });
            },
        };
        Ok(MapSerializer::new(key_type, value_type))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let element_types = match self.value_type {
            Type::Tuple(type_tuple::Tuple { name: Some(tuple_name), elements: type_tuple::Elements::Fields(fields) }) if tuple_name == name && fields.len() == len => fields,
            _ => {
                return Err(Error::UnexpectedValueType {
                    expected: self.value_type.clone(),
                    actual: format!("a struct type of size {len}"),
                });
            },
        };
        Ok(TupleSerializer::new(Some(name.to_string()), element_types))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!("enums are not yet supported as dynamic values")
    }
}

pub struct ListSerializer<'t> {
    value_type: &'t Type,
    elements: Vec<AnyValue>,
}

impl<'t> ListSerializer<'t> {
    fn new(value_type: &'t Type) -> Self {
        Self {
            value_type,
            elements: Vec::new(),
        }
    }
}

impl<'t> serde::ser::SerializeSeq for ListSerializer<'t> {
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = to_any_value(value, self.value_type)?;
        self.elements.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::List{ value_type: *self.value_type, list: self.elements })
    }
}

pub struct MapSerializer<'t> {
    elements: Vec<(AnyValue, AnyValue)>,
    value_type: &'t Type,
    key: Option<AnyValue>,
    key_type: &'t Type,
}

impl<'t> MapSerializer<'t> {
    fn new(key_type: &'t Type, value_type: &'t Type) -> Self {
        Self {
            elements: Vec::new(),
            key_type,
            key: None,
            value_type,
        }
    }
}

impl<'t> serde::ser::SerializeMap for MapSerializer<'t> {
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = to_any_value(key, self.key_type)?;
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
        let value = to_any_value(value, self.value_type)?;
        self.elements.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(AnyValue::Map { key_type: *self.key_type, value_type: *self.value_type, map: self.elements })
    }
}

pub struct TupleSerializer<V, I> {
    name: Option<String>,
    elements: Vec<V>,
    element_types: I,
}

impl<'t, V, I> TupleSerializer<V, I> where I: Iterator {
    fn new<E>(name: Option<String>, element_types: E) -> Self where E: IntoIterator<IntoIter = I>{
        Self {
            name,
            elements: Vec::new(),
            element_types: element_types.into_iter(),
        }
    }

    fn into_value(self) -> AnyValue
    where
        value_tuple::Elements: FromIterator<V>,
    {
        AnyValue::Tuple(value_tuple::Tuple {
            name: self.name,
            elements: value_tuple::Elements::from_iter(self.elements),
        })
    }
}

impl<'t, I> TupleSerializer<AnyValue, I> where I: Iterator<Item = &'t Type> {
    fn add_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let value_type = match self.element_types.next() {
            Some(t) => t,
            None => unreachable!(),
        };
        let element = to_any_value(value, value_type)?;
        self.elements.push(element);
        Ok(())
    }
}

impl<'t, I> TupleSerializer<value_tuple::Field, I> where I: Iterator<Item = &'t type_tuple::Field> {
    fn add_field<T: ?Sized>(&mut self, name: &str, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let name = name.to_string();
        let value_type = match self.element_types.next() {
            Some(t) => t,
            None => unreachable!(),
        };
        let element = to_any_value(value, &value_type.element)?;
        self.elements.push(value_tuple::Field {
            name,
            element,
        });
        Ok(())
    }
}

impl<'t, I> serde::ser::SerializeTuple for TupleSerializer<AnyValue, I> where I: Iterator<Item = &'t Type> {
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {

        self.add_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

impl<'t, I> serde::ser::SerializeTupleStruct for TupleSerializer<AnyValue, I>  where I: Iterator<Item = &'t Type> {
    type Ok = AnyValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.add_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

impl<'t, I> serde::ser::SerializeStruct for TupleSerializer<value_tuple::Field, I> where I: Iterator<Item = &'t type_tuple::Field> {
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
        self.add_field(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.into_value())
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {
    #[error("unexpected value type, expected {expected} but got {actual}", expected = expected)]
    UnexpectedValueType { expected: Type, actual: String },

    #[error("error: {0}")]
    Custom(String),
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}
