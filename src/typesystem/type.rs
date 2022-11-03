use indexmap::{indexmap, IndexMap};

#[derive(Debug, Default, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
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
    Tuple {
        elements: Vec<Type>,
    },
    TupleStruct {
        name: String,
        elements: Vec<Type>,
    },
    Struct {
        name: String,
        fields: IndexMap<String, Type>,
    },
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

    pub fn unit_tuple() -> Self {
        Self::Tuple {
            elements: Vec::new(),
        }
    }

    pub fn tuple<E>(elements: E) -> Self
    where
        E: Into<Vec<Type>>,
    {
        Self::Tuple {
            elements: elements.into(),
        }
    }

    pub fn tuple_struct<S, E>(name: S, elements: E) -> Self
    where
        S: Into<String>,
        E: Into<Vec<Type>>,
    {
        Self::TupleStruct {
            name: name.into(),
            elements: elements.into(),
        }
    }

    pub fn structure<S, F>(name: S, fields: F) -> Self
    where
        S: Into<String>,
        F: Into<IndexMap<String, Type>>,
    {
        Self::Struct {
            name: name.into(),
            fields: fields.into(),
        }
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

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

// TODO: type! macro ?

#[cfg(test)]
mod tests {
    use super::*;

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
            Type::tuple(vec![Type::Int32, Type::Float, Type::String]),
            Type::Tuple {
                elements: vec![Type::Int32, Type::Float, Type::String,],
            }
        );
    }

    #[test]
    fn test_type_tuple_struct() {
        assert_eq!(
            Type::tuple_struct("S", vec![Type::Int32, Type::Float]),
            Type::TupleStruct {
                name: "S".into(),
                elements: vec![Type::Int32, Type::Float]
            }
        );
    }

    #[test]
    fn test_type_structure() {
        assert_eq!(
            Type::structure(
                "S",
                IndexMap::from_iter([("a".into(), Type::Int32), ("b".into(), Type::Float)])
            ),
            Type::Struct {
                name: "S".into(),
                fields: indexmap! {
                    "a".into() => Type::Int32,
                    "b".into() => Type::Float
                }
            }
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
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &Type::list(Type::option(Type::Double)),
            &[
                Token::NewtypeVariant {
                    name: "Type",
                    variant: "List",
                },
                Token::NewtypeVariant {
                    name: "Type",
                    variant: "Option",
                },
                Token::UnitVariant {
                    name: "Type",
                    variant: "Double",
                },
            ],
        );
    }
}
