// TODO: remove the conversions module.
// mod conversions;
// pub use conversions::ToType;
pub mod tuple {
    use super::Type;
    use crate::typesystem::tuple;
    pub type Tuple = tuple::Tuple<Type>;
    pub type Elements = tuple::Elements<Type>;
    pub type Field = tuple::Field<Type>;
}
pub use tuple::Tuple;

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Type {
    #[default]
    None,
    Unknown,
    Void,
    Bool,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float,
    Double,
    String,
    Raw,
    Object,
    Dynamic,
    Option(Box<Type>),
    List(Box<Type>),
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
    Tuple(Tuple),
    VarArgs(Box<Type>),
    KwArgs(Box<Type>),
}

impl Type {
    pub fn list<T>(t: T) -> Self
    where
        T: Into<Box<Self>>,
    {
        Self::List(t.into())
    }

    pub fn map<K, V>(key: K, value: V) -> Self
    where
        K: Into<Box<Self>>,
        V: Into<Box<Self>>,
    {
        Self::Map {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn tuple<I>(elements: I) -> Self
    where
        I: IntoIterator,
        tuple::Elements: FromIterator<I::Item>,
    {
        Tuple::new(tuple::Elements::from_iter(elements)).into()
    }

    pub fn named_tuple<S, I>(name: S, fields: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator,
        tuple::Elements: FromIterator<I::Item>,
    {
        Tuple::named(name, tuple::Elements::from_iter(fields)).into()
    }

    pub fn var_args<T>(t: T) -> Self
    where
        T: Into<Box<Self>>,
    {
        Self::VarArgs(t.into())
    }

    pub fn kw_args<T>(t: T) -> Self
    where
        T: Into<Box<Self>>,
    {
        Self::KwArgs(t.into())
    }

    pub fn option<T>(t: T) -> Self
    where
        T: Into<Box<Self>>,
    {
        Self::Option(t.into())
    }
}

impl From<Tuple> for Type {
    fn from(t: Tuple) -> Self {
        Self::Tuple(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_type_list() {
        assert_eq!(Type::list(Type::String), Type::List(Box::new(Type::String)));
    }

    #[test]
    fn test_type_map() {
        assert_eq!(
            Type::map(Type::String, Type::UInt8,),
            Type::Map {
                key: Box::new(Type::String),
                value: Box::new(Type::UInt8)
            }
        );
    }

    #[test]
    fn test_type_tuple() {
        assert_eq!(
            Type::tuple([Type::Int32, Type::Float, Type::String]),
            Type::Tuple(Tuple {
                name: None,
                elements: tuple::Elements::Raw(vec![Type::Int32, Type::Float, Type::String,]),
            })
        );
        assert_eq!(
            Type::tuple([
                tuple::Field::new("i", Type::Int32),
                tuple::Field::new("f", Type::Float)
            ]),
            Type::Tuple(Tuple {
                name: None,
                elements: tuple::Elements::Fields(vec![
                    tuple::Field::new("i", Type::Int32),
                    tuple::Field::new("f", Type::Float)
                ])
            })
        );
    }

    #[test]
    fn test_type_named_tuple() {
        assert_eq!(
            Type::named_tuple(
                "S",
                [
                    tuple::Field::new("a", Type::Int32),
                    tuple::Field::new("b", Type::Float)
                ]
            ),
            Type::Tuple(Tuple {
                name: Some("S".into()),
                elements: tuple::Elements::from_iter([
                    tuple::Field::new("a", Type::Int32),
                    tuple::Field::new("b", Type::Float)
                ]),
            })
        );
        assert_eq!(
            Type::named_tuple("S", [Type::Int32, Type::Float]),
            Type::Tuple(Tuple {
                name: Some("S".into()),
                elements: tuple::Elements::from_iter([Type::Int32, Type::Float]),
            })
        );
    }

    #[test]
    fn test_type_var_args() {
        assert_eq!(
            Type::var_args(Type::list(Type::String)),
            Type::VarArgs(Box::new(Type::List(Box::new(Type::String))))
        );
    }

    #[test]
    fn test_type_kw_args() {
        assert_eq!(Type::kw_args(Type::Raw), Type::KwArgs(Box::new(Type::Raw)));
    }

    #[test]
    fn test_type_ser_de() {
        todo!()
    }
}
