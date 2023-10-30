use super::{StructField, TupleType, Type};
use crate::{list_ty, map_ty, struct_ty, variant_ty};
use std::marker::PhantomData;

#[derive(Debug)]
pub(crate) struct TypeSerializer<E>(PhantomData<E>);

impl<E> TypeSerializer<E> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<E> serde::Serializer for TypeSerializer<E>
where
    E: serde::ser::Error,
{
    type Ok = Type;
    type Error = E;

    type SerializeSeq = TypeSeqSerializer<E>;
    type SerializeTuple = TypeTupleSerializer<E>;
    type SerializeTupleStruct = TypeTupleStructSerializer<E>;
    type SerializeTupleVariant = TypeTupleSerializer<E>;
    type SerializeMap = TypeMapSerializer<E>;
    type SerializeStruct = TypeStructSerializer<E>;
    type SerializeStructVariant = TypeStructSerializer<E>;

    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Bool)
    }

    fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Int8)
    }

    fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Int16)
    }

    fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Int32)
    }

    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Int64)
    }

    fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Type::UInt8)
    }

    fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Type::UInt16)
    }

    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Type::UInt32)
    }

    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Type::UInt64)
    }

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Float32)
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Float64)
    }

    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Ok(Type::String)
    }

    fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Type::String)
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Raw)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Option(None))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value_type = value.serialize(self)?;
        Ok(Type::Option(Some(Box::new(value_type))))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Type::Unit)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(struct_ty!(name {}))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(variant_ty!(Type::Unit))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value_type = value.serialize(self)?;
        Ok(struct_ty!(name { value_type }))
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let value_type = value.serialize(self)?;
        Ok(variant_ty!(value_type))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(TypeSeqSerializer::new())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TypeTupleSerializer::new())
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TypeTupleStructSerializer::new(name))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TypeTupleSerializer::new())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(TypeMapSerializer::new())
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(TypeStructSerializer::new(name))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(TypeStructSerializer::new(name))
    }
}

#[derive(Debug)]
pub(crate) struct TypeSeqSerializer<E> {
    common_type: Option<Option<Type>>,
    phantom: PhantomData<E>,
}

impl<E> TypeSeqSerializer<E> {
    fn new() -> Self {
        Self {
            common_type: None,
            phantom: PhantomData,
        }
    }
}

impl<E> serde::ser::SerializeSeq for TypeSeqSerializer<E>
where
    E: serde::ser::Error,
{
    type Ok = Type;
    type Error = E;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        update_common_type(&mut self.common_type, value)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let common_type = self.common_type.flatten();
        Ok(list_ty!(common_type))
    }
}

fn update_common_type<T, E>(common_type: &mut Option<Option<Type>>, value: &T) -> Result<(), E>
where
    T: serde::Serialize + ?Sized,
    E: serde::ser::Error,
{
    let value_type = value.serialize(TypeSerializer::new())?;
    match &common_type {
        Some(Some(ty)) => {
            if ty != &value_type {
                *common_type = Some(None)
            }
        }
        Some(None) => { /* nothing */ }
        None => *common_type = Some(Some(value_type)),
    };
    Ok(())
}

#[derive(Debug)]
pub(crate) struct TypeMapSerializer<E> {
    common_key_type: Option<Option<Type>>,
    common_value_type: Option<Option<Type>>,
    phantom: PhantomData<E>,
}

impl<E> TypeMapSerializer<E> {
    fn new() -> Self {
        Self {
            common_key_type: None,
            common_value_type: None,
            phantom: PhantomData,
        }
    }
}

impl<E> serde::ser::SerializeMap for TypeMapSerializer<E>
where
    E: serde::ser::Error,
{
    type Ok = Type;
    type Error = E;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        update_common_type(&mut self.common_key_type, key)?;
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        update_common_type(&mut self.common_value_type, value)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let key_type = self.common_key_type.flatten();
        let value_type = self.common_value_type.flatten();
        Ok(map_ty!(key_type, value_type))
    }
}

#[derive(Debug)]
pub(crate) struct TypeTupleSerializer<E> {
    types: Vec<Option<Type>>,
    phantom: PhantomData<E>,
}

impl<E> TypeTupleSerializer<E> {
    fn new() -> Self {
        Self {
            types: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<E> serde::ser::SerializeTuple for TypeTupleSerializer<E>
where
    E: serde::ser::Error,
{
    type Ok = Type;
    type Error = E;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value_type = value.serialize(TypeSerializer::new())?;
        self.types.push(Some(value_type));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let tuple_ty = TupleType::Tuple(self.types);
        let ty = Type::Tuple(tuple_ty);
        Ok(ty)
    }
}

impl<E> serde::ser::SerializeTupleVariant for TypeTupleSerializer<E>
where
    E: serde::ser::Error,
{
    type Ok = Type;
    type Error = E;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        <Self as serde::ser::SerializeTuple>::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let tuple_ty = TupleType::Tuple(self.types);
        let ty = Type::Tuple(tuple_ty);
        Ok(variant_ty!(ty))
    }
}

#[derive(Debug)]
pub(crate) struct TypeStructSerializer<E> {
    name: &'static str,
    fields: Vec<StructField>,
    phantom: PhantomData<E>,
}

impl<E> TypeStructSerializer<E> {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            fields: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<E> serde::ser::SerializeStruct for TypeStructSerializer<E>
where
    E: serde::ser::Error,
{
    type Ok = Type;
    type Error = E;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value_type = value.serialize(TypeSerializer::new())?;
        self.fields.push(StructField {
            name: key.to_owned(),
            value_type: Some(value_type),
        });
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let struct_ty = TupleType::Struct(self.name.to_owned(), self.fields);
        let ty = Type::Tuple(struct_ty);
        Ok(ty)
    }
}

#[derive(Debug)]
pub(crate) struct TypeTupleStructSerializer<E> {
    name: &'static str,
    fields: Vec<Option<Type>>,
    phantom: PhantomData<E>,
}

impl<E> TypeTupleStructSerializer<E> {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            fields: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<E> serde::ser::SerializeTupleStruct for TypeTupleStructSerializer<E>
where
    E: serde::ser::Error,
{
    type Ok = Type;
    type Error = E;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value_type = value.serialize(TypeSerializer::new())?;
        self.fields.push(Some(value_type));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let struct_ty = TupleType::TupleStruct(self.name.to_owned(), self.fields);
        let ty = Type::Tuple(struct_ty);
        Ok(ty)
    }
}

impl<E> serde::ser::SerializeStructVariant for TypeStructSerializer<E>
where
    E: serde::ser::Error,
{
    type Ok = Type;
    type Error = E;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
