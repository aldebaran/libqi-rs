use indexmap::IndexMap;

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
    Tuple(Vec<Type>),
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

impl std::fmt::Display for Type {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

// TODO: type! macro ?

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_ser_de() {
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &Type::List(Type::Option(Type::Double.into()).into()),
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
