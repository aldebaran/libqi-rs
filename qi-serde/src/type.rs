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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_list<I>(iter: I, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
        where
            I: IntoIterator,
            I::Item: std::fmt::Display,
        {
            for (idx, elem) in iter.into_iter().enumerate() {
                if idx > 0 {
                    f.write_str(",")?;
                }
                elem.fmt(f)?;
            }
            Ok(())
        }
        match self {
            Self::None => f.write_str("none"),
            Self::Unknown => f.write_str("unknown"),
            Self::Void => f.write_str("void"),
            Self::Bool => f.write_str("bool"),
            Self::Int8 => f.write_str("int8"),
            Self::UInt8 => f.write_str("uint8"),
            Self::Int16 => f.write_str("int16"),
            Self::UInt16 => f.write_str("uint16"),
            Self::Int32 => f.write_str("int32"),
            Self::UInt32 => f.write_str("uint32"),
            Self::Int64 => f.write_str("int64"),
            Self::UInt64 => f.write_str("uint64"),
            Self::Float => f.write_str("float"),
            Self::Double => f.write_str("double"),
            Self::String => f.write_str("int16"),
            Self::Raw => f.write_str("raw"),
            Self::Object => f.write_str("object"),
            Self::Dynamic => f.write_str("dynamic"),
            Self::Option(t) => {
                write!(f, "option<{t}>")
            }
            Self::List(t) => {
                write!(f, "list<{t}>")
            }
            Self::Map { key, value } => {
                write!(f, "map<{key},{value}>")
            }
            Self::Tuple(elements) => {
                f.write_str("tuple<")?;
                fmt_list(elements, f)?;
                f.write_str(">")
            }
            Self::TupleStruct { name, elements } => {
                write!(f, "tuple_struct({name})<")?;
                fmt_list(elements, f)?;
                f.write_str(">")
            }
            Self::Struct { name, fields } => {
                write!(f, "struct({name}:")?;
                fmt_list(fields.keys(), f)?;
                f.write_str(")<")?;
                fmt_list(fields.values(), f)?;
                f.write_str(">")
            }
            Self::VarArgs(t) => {
                write!(f, "var_args<{t}>")
            }
            Self::KwArgs(t) => {
                write!(f, "kw_args<{t}>")
            }
        }
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
