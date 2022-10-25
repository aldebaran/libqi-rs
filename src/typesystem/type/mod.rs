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

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
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
    const CHAR_NONE: char = '_';
    const CHAR_UNKNOWN: char = 'X';
    const CHAR_VOID: char = 'v';
    const CHAR_BOOL: char = 'b';
    const CHAR_INT8: char = 'c';
    const CHAR_UINT8: char = 'C';
    const CHAR_INT16: char = 'w';
    const CHAR_UINT16: char = 'W';
    const CHAR_INT32: char = 'i';
    const CHAR_UINT32: char = 'I';
    const CHAR_INT64: char = 'l';
    const CHAR_UINT64: char = 'L';
    const CHAR_FLOAT: char = 'f';
    const CHAR_DOUBLE: char = 'd';
    const CHAR_STRING: char = 's';
    const CHAR_RAW: char = 'r';
    const CHAR_OBJECT: char = 'o';
    const CHAR_DYNAMIC: char = 'm';
    const CHAR_MARK_OPTION: char = '+';
    const CHAR_LIST_BEGIN: char = '[';
    const CHAR_LIST_END: char = ']';
    const CHAR_MAP_BEGIN: char = '{';
    const CHAR_MAP_END: char = '}';
    const CHAR_TUPLE_BEGIN: char = '(';
    const CHAR_TUPLE_END: char = ')';
    const CHAR_MARK_VARSARGS: char = '#';
    const CHAR_MARK_KWARGS: char = '~';
    const CHAR_ANNOTATIONS_BEGIN: char = '<';
    const CHAR_ANNOTATIONS_SEP: char = ',';
    const CHAR_ANNOTATIONS_END: char = '>';

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
        I: IntoIterator<Item = Self>,
    {
        Tuple::anonymous(tuple::Elements::from_iter(elements)).into()
    }

    pub fn structure<S, I, F>(name: S, fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        S: Into<String>,
        F: Into<tuple::Field>,
    {
        let elements = tuple::Elements::from_iter(fields.into_iter().map(Into::into));
        Tuple::named(name, elements).into()
    }

    // TODO: tuple_struct

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

    fn parse(iter: &mut std::str::Chars) -> Result<Self, FromStrError> {
        let input = iter.as_str();
        let c = iter.next().ok_or(FromStrError::EndOfInput)?;
        match c {
            Self::CHAR_NONE => Ok(Self::None),
            Self::CHAR_UNKNOWN => Ok(Self::Unknown),
            Self::CHAR_VOID => Ok(Self::Void),
            Self::CHAR_BOOL => Ok(Self::Bool),
            Self::CHAR_INT8 => Ok(Self::Int8),
            Self::CHAR_UINT8 => Ok(Self::UInt8),
            Self::CHAR_INT16 => Ok(Self::Int16),
            Self::CHAR_UINT16 => Ok(Self::UInt16),
            Self::CHAR_INT32 => Ok(Self::Int32),
            Self::CHAR_UINT32 => Ok(Self::UInt32),
            Self::CHAR_INT64 => Ok(Self::Int64),
            Self::CHAR_UINT64 => Ok(Self::UInt64),
            Self::CHAR_FLOAT => Ok(Self::Float),
            Self::CHAR_DOUBLE => Ok(Self::Double),
            Self::CHAR_STRING => Ok(Self::String),
            Self::CHAR_RAW => Ok(Self::Raw),
            Self::CHAR_OBJECT => Ok(Self::Object),
            Self::CHAR_DYNAMIC => Ok(Self::Dynamic),
            Self::CHAR_MARK_OPTION => Self::parse_option_tail(iter, input),
            Self::CHAR_LIST_BEGIN => Self::parse_list_tail(iter, input),
            Self::CHAR_MAP_BEGIN => Self::parse_map_tail(iter, input),
            Self::CHAR_TUPLE_BEGIN => {
                let tuple = Self::parse_tuple_tail(iter, input)?;
                match iter.next() {
                    Some(Self::CHAR_ANNOTATIONS_BEGIN) => {
                        let annotations = Self::parse_tuple_annotations_tail(iter, input);
                        todo!()
                    }
                    _ => Ok(tuple),
                }
            }
            Self::CHAR_MARK_VARSARGS => Self::parse_varargs_tail(iter, input),
            Self::CHAR_MARK_KWARGS => Self::parse_kwargs_tail(iter, input),
            _ => Err(FromStrError::UnexpectedChar(c, input.into())),
        }
    }

    fn parse_option_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        match Self::parse(iter) {
            Ok(t) => Ok(Self::option(t)),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingOptionValueType(start.into()),
                _ => FromStrError::OptionValueTypeParsing(Box::new(err)),
            }),
        }
    }

    fn parse_varargs_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        match Self::parse(iter) {
            Ok(t) => Ok(Self::var_args(t)),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingVarArgsValueType(start.into()),
                _ => FromStrError::VarArgsValueTypeParsing(Box::new(err)),
            }),
        }
    }

    fn parse_kwargs_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        match Self::parse(iter) {
            Ok(t) => Ok(Self::kw_args(t)),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingKwArgsValueType(start.into()),
                _ => FromStrError::KwArgsValueTypeParsing(Box::new(err)),
            }),
        }
    }

    fn parse_list_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        let t = Self::parse(iter).map_err(|err| match err {
            FromStrError::UnexpectedChar(Self::CHAR_LIST_END, _) | FromStrError::EndOfInput => {
                FromStrError::MissingListValueType(start.into())
            }
            _ => FromStrError::ListValueTypeParsing(Box::new(err)),
        })?;
        match iter.next() {
            Some(Self::CHAR_LIST_END) => Ok(Self::list(t)),
            _ => Err(FromStrError::MissingListEnd(start.into())),
        }
    }

    fn parse_map_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        let key = Self::parse(iter).map_err(|err| match err {
            FromStrError::UnexpectedChar(Self::CHAR_MAP_END, _) | FromStrError::EndOfInput => {
                FromStrError::MissingMapKeyType(start.into())
            }
            _ => FromStrError::MapKeyTypeParsing(Box::new(err)),
        })?;
        let value = Self::parse(iter).map_err(|err| match err {
            FromStrError::UnexpectedChar(Self::CHAR_MAP_END, _) => {
                FromStrError::MissingMapValueType(start.into())
            }
            _ => FromStrError::MapValueTypeParsing(Box::new(err)),
        })?;
        match iter.next() {
            Some(Self::CHAR_MAP_END) => Ok(Self::map(key, value)),
            _ => Err(FromStrError::MissingMapEnd(start.into())),
        }
    }

    fn parse_tuple_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        let mut fields = Vec::new();
        loop {
            match Self::parse(iter) {
                Ok(t) => fields.push(t),
                Err(err) => {
                    break match err {
                        FromStrError::UnexpectedChar(Self::CHAR_TUPLE_END, _) => {
                            Ok(Self::tuple(fields))
                        }
                        FromStrError::EndOfInput => {
                            Err(FromStrError::MissingTupleEnd(start.into()))
                        }
                        _ => Err(FromStrError::TupleElementTypeParsing(Box::new(err))),
                    }
                }
            }
        }
    }

    fn parse_tuple_annotations_tail(
        iter: &mut std::str::Chars,
        start: &str,
    ) -> Result<Self, FromStrError> {
        todo!()
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        match self {
            Type::None => f.write_char(Self::CHAR_NONE),
            Type::Unknown => f.write_char(Self::CHAR_UNKNOWN),
            Type::Void => f.write_char(Self::CHAR_VOID),
            Type::Bool => f.write_char(Self::CHAR_BOOL),
            Type::Int8 => f.write_char(Self::CHAR_INT8),
            Type::UInt8 => f.write_char(Self::CHAR_UINT8),
            Type::Int16 => f.write_char(Self::CHAR_INT16),
            Type::UInt16 => f.write_char(Self::CHAR_UINT16),
            Type::Int32 => f.write_char(Self::CHAR_INT32),
            Type::UInt32 => f.write_char(Self::CHAR_UINT32),
            Type::Int64 => f.write_char(Self::CHAR_INT64),
            Type::UInt64 => f.write_char(Self::CHAR_UINT64),
            Type::Float => f.write_char(Self::CHAR_FLOAT),
            Type::Double => f.write_char(Self::CHAR_DOUBLE),
            Type::String => f.write_char(Self::CHAR_STRING),
            Type::Raw => f.write_char(Self::CHAR_RAW),
            Type::Object => f.write_char(Self::CHAR_OBJECT),
            Type::Dynamic => f.write_char(Self::CHAR_DYNAMIC),
            Type::Option(o) => write!(f, "{mark}{o}", mark = Self::CHAR_MARK_OPTION),
            Type::List(t) => write!(
                f,
                "{beg}{t}{end}",
                beg = Self::CHAR_LIST_BEGIN,
                end = Self::CHAR_LIST_END
            ),
            Type::Map { key, value } => write!(
                f,
                "{beg}{key}{value}{end}",
                beg = Self::CHAR_MAP_BEGIN,
                end = Self::CHAR_MAP_END
            ),
            Type::Tuple(Tuple { name, elements }) => write!(
                f,
                "{beg}{ts}{end}",
                beg = Self::CHAR_TUPLE_BEGIN,
                end = Self::CHAR_TUPLE_END,
                ts = elements
                    .into_iter()
                    .fold(String::new(), |s, t| s + &t.to_string())
            ),
            Type::VarArgs(t) => write!(f, "{mark}{t}", mark = Self::CHAR_MARK_VARSARGS),
            Type::KwArgs(t) => write!(f, "{mark}{t}", mark = Self::CHAR_MARK_KWARGS),
        }
    }
}

impl From<Tuple> for Type {
    fn from(t: Tuple) -> Self {
        Self::Tuple(t)
    }
}

impl std::str::FromStr for Type {
    type Err = FromStrError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Self::parse(&mut src.chars())
    }
}

#[derive(thiserror::Error, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum FromStrError {
    #[default]
    #[error("end of input reached")]
    EndOfInput,

    #[error("unexpected character \'{0}\' in input \"{1}\"")]
    UnexpectedChar(char, String),

    #[error("value type of option starting at input \"{0}\" is missing")]
    MissingOptionValueType(String),

    #[error("parsing of option value type failed")]
    OptionValueTypeParsing(#[source] Box<FromStrError>),

    #[error("value type of varargs starting at input \"{0}\" is missing")]
    MissingVarArgsValueType(String),

    #[error("parsing of varargs value type failed")]
    VarArgsValueTypeParsing(#[source] Box<FromStrError>),

    #[error("value type of kwargs starting at input \"{0}\" is missing")]
    MissingKwArgsValueType(String),

    #[error("parsing of kwargs value type failed")]
    KwArgsValueTypeParsing(#[source] Box<FromStrError>),

    #[error("value type of list starting at input \"{0}\" is missing")]
    MissingListValueType(String),

    #[error("parsing of list value type failed")]
    ListValueTypeParsing(#[source] Box<FromStrError>),

    #[error("end of list starting at input \"{0}\" is missing")]
    MissingListEnd(String),

    #[error("key type of map starting at input \"{0}\" is missing")]
    MissingMapKeyType(String),

    #[error("parsing of map key type failed")]
    MapKeyTypeParsing(#[source] Box<FromStrError>),

    #[error("value type of map starting at input \"{0}\" is missing")]
    MissingMapValueType(String),

    #[error("parsing of map value type failed")]
    MapValueTypeParsing(#[source] Box<FromStrError>),

    #[error("end of map starting at input \"{0}\" is missing")]
    MissingMapEnd(String),

    #[error("parsing of a tuple element type failed")]
    TupleElementTypeParsing(#[source] Box<FromStrError>),

    #[error("end of tuple starting at input \"{0}\" is missing")]
    MissingTupleEnd(String),

    #[error("annotation for structure name of tuple starting at input \"{0}\" is missing")]
    MissingTupleAnnotationStructName(String),

    #[error("annotation for structure field name of tuple starting at input \"{0}\" is missing")]
    MissingTupleAnnotationFieldName(String),

    #[error("unexpected annotation for structure field name at \"{1}\" of tuple starting at input \"{2}\" (expected {0} fields)")]
    UnexpectedTupleAnnotationFieldName(usize, String, String),

    #[error("end of annotation for tuple starting at input \"{0}\" is missing")]
    MissingTupleAnnotationEnd(String),
}

impl serde::Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Type {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <&str>::deserialize(deserializer)?
            .parse()
            .map_err(|e| serde::de::Error::custom(e))
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
    }

    #[test]
    fn test_type_structure() {
        assert_eq!(
            Type::structure(
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
    fn test_type_to_from_string() {
        macro_rules! assert_to_from_str {
            ($t:expr, $s:expr) => {{
                assert_eq!(
                    $t.to_string(),
                    $s,
                    "Left is {t:?}.to_string(), Right is {s:?}",
                    t = $t,
                    s = $s
                );
                assert_eq!(
                    $s.parse::<Type>(),
                    Ok($t),
                    "Left is {s:?}.parse(), Right is {t:?}",
                    s = $s,
                    t = $t
                );
            }};
        }
        assert_to_from_str!(Type::None, "_");
        assert_to_from_str!(Type::Unknown, "X");
        assert_to_from_str!(Type::Void, "v");
        assert_to_from_str!(Type::Bool, "b");
        assert_to_from_str!(Type::Int8, "c");
        assert_to_from_str!(Type::UInt8, "C");
        assert_to_from_str!(Type::Int16, "w");
        assert_to_from_str!(Type::UInt16, "W");
        assert_to_from_str!(Type::Int32, "i");
        assert_to_from_str!(Type::UInt32, "I");
        assert_to_from_str!(Type::Int64, "l");
        assert_to_from_str!(Type::UInt64, "L");
        assert_to_from_str!(Type::Float, "f");
        assert_to_from_str!(Type::Double, "d");
        assert_to_from_str!(Type::String, "s");
        assert_to_from_str!(Type::Raw, "r");
        assert_to_from_str!(Type::Object, "o");
        assert_to_from_str!(Type::Dynamic, "m");
        assert_to_from_str!(Type::option(Type::Void), "+v");
        assert_to_from_str!(Type::list(Type::Int32), "[i]");
        assert_to_from_str!(Type::map(Type::Float, Type::String), "{fs}");
        assert_to_from_str!(
            Type::tuple([Type::Float, Type::String, Type::UInt32]),
            "(fsI)"
        );
        assert_to_from_str!(
            Type::structure(
                "ExplorationMap",
                [
                    tuple::Field::new(
                        "points",
                        Type::list(Type::structure(
                            "Point",
                            [
                                tuple::Field::new("x", Type::Double),
                                tuple::Field::new("y", Type::Double)
                            ],
                        )),
                    ),
                    tuple::Field::new("timestamp", Type::UInt64),
                ],
            ),
            "([(dd)<Point,x,y>]L)<ExplorationMap,points,timestamp>"
        );
        assert_to_from_str!(Type::var_args(Type::Dynamic), "#m");
        assert_to_from_str!(Type::kw_args(Type::Object), "~o");
        // Some complex type for fun.
        assert_to_from_str!(
            Type::tuple([
                Type::list(Type::map(Type::option(Type::Object), Type::Raw)),
                Type::kw_args(Type::Double),
                Type::var_args(Type::option(Type::Dynamic)),
            ]),
            "([{+or}]~d#+m)"
        );
    }

    #[test]
    fn test_type_from_str_errors() {
        assert_eq!("".parse::<Type>(), Err(FromStrError::EndOfInput));
        assert_eq!(
            "u".parse::<Type>(),
            Err(FromStrError::UnexpectedChar('u', "u".into()))
        );
        // Option
        assert_eq!(
            "+".parse::<Type>(),
            Err(FromStrError::MissingOptionValueType("+".into()))
        );
        assert_eq!(
            "+[".parse::<Type>(),
            Err(FromStrError::OptionValueTypeParsing(Box::new(
                FromStrError::MissingListValueType("[".into())
            )))
        );
        // VarArgs
        assert_eq!(
            "#".parse::<Type>(),
            Err(FromStrError::MissingVarArgsValueType("#".into()))
        );
        assert_eq!(
            "#[".parse::<Type>(),
            Err(FromStrError::VarArgsValueTypeParsing(Box::new(
                FromStrError::MissingListValueType("[".into())
            )))
        );
        // KwArgs
        assert_eq!(
            "~".parse::<Type>(),
            Err(FromStrError::MissingKwArgsValueType("~".into()))
        );
        assert_eq!(
            "~[".parse::<Type>(),
            Err(FromStrError::KwArgsValueTypeParsing(Box::new(
                FromStrError::MissingListValueType("[".into())
            )))
        );
        // Lists
        assert_eq!(
            "[".parse::<Type>(),
            Err(FromStrError::MissingListValueType("[".into()))
        );
        assert_eq!(
            "[]".parse::<Type>(),
            Err(FromStrError::MissingListValueType("[]".into()))
        );
        assert_eq!(
            "[i".parse::<Type>(),
            Err(FromStrError::MissingListEnd("[i".into()))
        );
        assert_eq!(
            "[{i}]".parse::<Type>(),
            Err(FromStrError::ListValueTypeParsing(Box::new(
                FromStrError::MissingMapValueType("{i}]".into())
            )))
        );
        // The error is `UnexpectedChar` and not `MissingTupleEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "[(]".parse::<Type>(),
            Err(FromStrError::ListValueTypeParsing(Box::new(
                FromStrError::TupleElementTypeParsing(Box::new(FromStrError::UnexpectedChar(
                    ']',
                    "]".into()
                )))
            )))
        );
        // Maps
        assert_eq!(
            "{".parse::<Type>(),
            Err(FromStrError::MissingMapKeyType("{".into()))
        );
        assert_eq!(
            "{}".parse::<Type>(),
            Err(FromStrError::MissingMapKeyType("{}".into()))
        );
        assert_eq!(
            "{i}".parse::<Type>(),
            Err(FromStrError::MissingMapValueType("{i}".into()))
        );
        assert_eq!(
            "{ii".parse::<Type>(),
            Err(FromStrError::MissingMapEnd("{ii".into()))
        );
        assert_eq!(
            "{[]i}".parse::<Type>(),
            Err(FromStrError::MapKeyTypeParsing(Box::new(
                FromStrError::MissingListValueType("[]i}".into())
            )))
        );
        assert_eq!(
            "{i[]}".parse::<Type>(),
            Err(FromStrError::MapValueTypeParsing(Box::new(
                FromStrError::MissingListValueType("[]}".into())
            )))
        );
        // The error is `UnexpectedChar` and not `MissingListEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "{i[}".parse::<Type>(),
            Err(FromStrError::MapValueTypeParsing(Box::new(
                FromStrError::ListValueTypeParsing(Box::new(FromStrError::UnexpectedChar(
                    '}',
                    "}".into()
                )))
            )))
        );
        // Tuples
        assert_eq!(
            "(".parse::<Type>(),
            Err(FromStrError::MissingTupleEnd("(".into()))
        );
        assert_eq!(
            "(iii".parse::<Type>(),
            Err(FromStrError::MissingTupleEnd("(iii".into()))
        );
        assert_eq!(
            "(i[i)".parse::<Type>(),
            Err(FromStrError::TupleElementTypeParsing(Box::new(
                FromStrError::MissingListEnd("[i)".into())
            )))
        );
        // Tuples annotations
        assert_eq!(
            "(i)<".parse::<Type>(),
            Err(FromStrError::MissingTupleAnnotationEnd("(i)<".into()))
        );
        assert_eq!(
            "(i)<>".parse::<Type>(),
            Err(FromStrError::MissingTupleAnnotationStructName(
                "(i)<>".into()
            ))
        );
        assert_eq!(
            "(i)<S>".parse::<Type>(),
            Err(FromStrError::MissingTupleAnnotationFieldName(
                "(i)<S>".into()
            ))
        );
        assert_eq!(
            "(i)<S,a,b>".parse::<Type>(),
            Err(FromStrError::UnexpectedTupleAnnotationFieldName(
                1,
                "b>".into(),
                "(i)<S,a,b>".into()
            ))
        );
        // The error is `UnexpectedChar` and not `MissingMapEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "(i{i)".parse::<Type>(),
            Err(FromStrError::TupleElementTypeParsing(Box::new(
                FromStrError::MapValueTypeParsing(Box::new(FromStrError::UnexpectedChar(
                    ')',
                    ")".into()
                )))
            )))
        );
    }

    #[test]
    fn test_type_from_str_meta_object() {
        let input = "({I(Issss[(ss)<MetaMethodParameter,name,description>]s)\
                     <MetaMethod,uid,returnSignature,name,parametersSignature,\
                     description,parameters,returnDescription>}{I(Iss)<MetaSignal,\
                     uid,name,signature>}{I(Iss)<MetaProperty,uid,name,signature>}s)\
                     <MetaObject,methods,signals,properties,description>";
        let t: Type = input.parse().unwrap();
        assert_eq!(
            t,
            Type::structure(
                "MetaObject",
                [
                    tuple::Field::new(
                        "methods",
                        Type::map(
                            Type::Int64,
                            Type::structure(
                                "MetaMethod",
                                [
                                    tuple::Field::new("uid", Type::Int64),
                                    tuple::Field::new("returnSignature", Type::String),
                                    tuple::Field::new("name", Type::String),
                                    tuple::Field::new("parametersSignature", Type::String),
                                    tuple::Field::new("description", Type::String),
                                    tuple::Field::new(
                                        "parameters",
                                        Type::list(Type::structure(
                                            "MetaMethodParameter",
                                            [
                                                tuple::Field::new("name", Type::String),
                                                tuple::Field::new("description", Type::String)
                                            ]
                                        ))
                                    ),
                                    tuple::Field::new("returnDescription", Type::String),
                                ]
                            )
                        )
                    ),
                    tuple::Field::new(
                        "signals",
                        Type::map(
                            Type::Int64,
                            Type::structure(
                                "MetaSignal",
                                [
                                    tuple::Field::new("uid", Type::Int64),
                                    tuple::Field::new("name", Type::String),
                                    tuple::Field::new("signature", Type::String),
                                ]
                            )
                        )
                    ),
                    tuple::Field::new(
                        "properties",
                        Type::map(
                            Type::Int64,
                            Type::structure(
                                "MetaProperty",
                                [
                                    tuple::Field::new("uid", Type::Int64),
                                    tuple::Field::new("name", Type::String),
                                    tuple::Field::new("signature", Type::String),
                                ]
                            )
                        )
                    ),
                    tuple::Field::new("description", Type::String)
                ]
            )
        );
    }

    #[test]
    fn test_type_ser_de() {
        assert_tokens(
            &Type::Tuple(Tuple::named(
                "Point",
                tuple::Elements::from_iter([
                    tuple::Field::new("x", Type::Double),
                    tuple::Field::new("y", Type::Double),
                ]),
            )),
            &[Token::Str("(dd)<Point,x,y>")],
        )
    }
}
