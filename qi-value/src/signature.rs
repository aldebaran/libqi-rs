use crate::{
    ty::{self, StructAnnotations, Tuple, Type},
    FromValue, FromValueError, IntoValue, Reflect, RuntimeReflect, ToValue, Value,
};
use serde_with::serde_as;

#[serde_as]
#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Hash,
    serde_with::SerializeDisplay,
    serde_with::DeserializeFromStr,
)]
pub struct Signature(pub(crate) Option<Type>);

impl Signature {
    pub fn new(t: Option<Type>) -> Self {
        Self(t)
    }

    pub fn dynamic() -> Self {
        Self(None)
    }

    pub fn to_type(&self) -> Option<&Type> {
        self.0.as_ref()
    }

    pub fn into_type(self) -> Option<Type> {
        self.0
    }
}

impl Reflect for Signature {
    fn ty() -> Option<Type> {
        Some(Type::String)
    }
}

impl RuntimeReflect for Signature {
    fn ty(&self) -> Type {
        Type::String
    }
}

impl FromValue<'_> for Signature {
    fn from_value(value: Value<'_>) -> Result<Self, FromValueError> {
        value
            .as_string()
            .and_then(|str| str.as_str())
            .ok_or_else(|| FromValueError::TypeMismatch {
                expected: "Signature".to_owned(),
                actual: value.to_string(),
            })
            .and_then(|str| {
                str.parse()
                    .map_err(|err: ParseError| FromValueError::Other(err.into()))
            })
    }
}

impl ToValue for Signature {
    fn to_value(&self) -> Value<'_> {
        Value::String(self.to_string().into())
    }
}

impl<'a> IntoValue<'a> for Signature {
    fn into_value(self) -> Value<'a> {
        Value::String(self.to_string().into())
    }
}

impl TryFrom<Value<'_>> for Signature {
    type Error = FromValueError;

    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        value.cast_into()
    }
}

impl From<Type> for Signature {
    fn from(t: Type) -> Self {
        Self(Some(t))
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_type(self.0.as_ref(), f)
    }
}

impl std::str::FromStr for Signature {
    type Err = ParseError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut iter = src.chars();
        let t = parse_type(&mut iter)?;
        Ok(Self(t))
    }
}

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
const CHAR_MARK_VAR_ARGS: char = '#';
const CHAR_ANNOTATIONS_BEGIN: char = '<';
const CHAR_ANNOTATIONS_SEP: char = ',';
const CHAR_ANNOTATIONS_END: char = '>';

fn write_type(t: Option<&Type>, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use std::fmt::Write;
    match t {
        None => f.write_char(CHAR_DYNAMIC),
        Some(t) => match t {
            Type::Unit => f.write_char(CHAR_VOID),
            Type::Bool => f.write_char(CHAR_BOOL),
            Type::Int8 => f.write_char(CHAR_INT8),
            Type::UInt8 => f.write_char(CHAR_UINT8),
            Type::Int16 => f.write_char(CHAR_INT16),
            Type::UInt16 => f.write_char(CHAR_UINT16),
            Type::Int32 => f.write_char(CHAR_INT32),
            Type::UInt32 => f.write_char(CHAR_UINT32),
            Type::Int64 => f.write_char(CHAR_INT64),
            Type::UInt64 => f.write_char(CHAR_UINT64),
            Type::Float32 => f.write_char(CHAR_FLOAT),
            Type::Float64 => f.write_char(CHAR_DOUBLE),
            Type::String => f.write_char(CHAR_STRING),
            Type::Raw => f.write_char(CHAR_RAW),
            Type::Object => f.write_char(CHAR_OBJECT),
            Type::Option(o) => {
                f.write_char(CHAR_MARK_OPTION)?;
                write_type(o.as_deref(), f)
            }
            Type::List(t) => {
                f.write_char(CHAR_LIST_BEGIN)?;
                write_type(t.as_deref(), f)?;
                f.write_char(CHAR_LIST_END)
            }
            Type::Map { key, value } => {
                f.write_char(CHAR_MAP_BEGIN)?;
                write_type(key.as_deref(), f)?;
                write_type(value.as_deref(), f)?;
                f.write_char(CHAR_MAP_END)
            }
            Type::Tuple(tuple) => {
                f.write_char(CHAR_TUPLE_BEGIN)?;
                for element in tuple.element_types() {
                    write_type(element.as_ref(), f)?;
                }
                f.write_char(CHAR_TUPLE_END)?;
                if let Some(annotations) = tuple.annotations() {
                    f.write_char(CHAR_ANNOTATIONS_BEGIN)?;
                    f.write_str(&annotations.name)?;
                    if let Some(fields) = &annotations.field_names {
                        for field in fields {
                            write!(f, ",{field}", field = field)?;
                        }
                    }
                    f.write_char(CHAR_ANNOTATIONS_END)?;
                }
                Ok(())
            }
            Type::VarArgs(t) => {
                f.write_char(CHAR_MARK_VAR_ARGS)?;
                write_type(t.as_deref(), f)
            }
        },
    }
}

fn advance_once<I>(mut iter: I)
where
    I: Iterator,
{
    if iter.next().is_none() {
        unreachable!(
            "the precondition over the presence of an element in the iterator is not verified"
        )
    }
}

fn parse_type(iter: &mut std::str::Chars) -> Result<Option<Type>, ParseError> {
    let type_str = iter.as_str();
    // Multiple characters types are read from the beginning. Therefore we clone the iterator,
    // read one char, and if we detect any marker of those types, pass the original iterator to
    // the subparsing function and return its result immediately.
    let c = iter.clone().next().ok_or(ParseError::EndOfInput)?;
    match c {
        CHAR_MARK_OPTION => return Ok(Some(parse_option(iter)?)),
        CHAR_MARK_VAR_ARGS => return Ok(Some(parse_var_args(iter)?)),
        CHAR_LIST_BEGIN => return Ok(Some(parse_list(iter)?)),
        CHAR_MAP_BEGIN => return Ok(Some(parse_map(iter)?)),
        CHAR_TUPLE_BEGIN => return Ok(Some(parse_tuple(iter)?)),
        _ => (),
    };
    // Now all that's left are simple character types, which we already have the value of.
    // Therefore we can advance the iterator by one.
    advance_once(iter.by_ref());
    let t = match c {
        CHAR_VOID => Some(Type::Unit),
        CHAR_BOOL => Some(Type::Bool),
        CHAR_INT8 => Some(Type::Int8),
        CHAR_UINT8 => Some(Type::UInt8),
        CHAR_INT16 => Some(Type::Int16),
        CHAR_UINT16 => Some(Type::UInt16),
        CHAR_INT32 => Some(Type::Int32),
        CHAR_UINT32 => Some(Type::UInt32),
        CHAR_INT64 => Some(Type::Int64),
        CHAR_UINT64 => Some(Type::UInt64),
        CHAR_FLOAT => Some(Type::Float32),
        CHAR_DOUBLE => Some(Type::Float64),
        CHAR_STRING => Some(Type::String),
        CHAR_RAW => Some(Type::Raw),
        CHAR_OBJECT => Some(Type::Object),
        CHAR_DYNAMIC => None,
        // Anything else is unexpected.
        c => return Err(ParseError::UnexpectedChar(c, type_str.to_owned())),
    };
    Ok(t)
}

fn parse_option(iter: &mut std::str::Chars) -> Result<Type, ParseError> {
    let option_str = iter.as_str();
    advance_once(iter.by_ref());
    let value_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                ParseError::EndOfInput => ParseError::MissingOptionValueType(option_str.to_owned()),
                _ => ParseError::OptionValueTypeParsing(Box::new(err)),
            })
        }
    };
    Ok(Type::Option(value_type.map(Box::new)))
}

fn parse_var_args(iter: &mut std::str::Chars) -> Result<Type, ParseError> {
    let var_args_str = iter.as_str();
    advance_once(iter.by_ref());
    let value_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                ParseError::EndOfInput => {
                    ParseError::MissingVarArgsValueType(var_args_str.to_owned())
                }
                _ => ParseError::VarArgsValueTypeParsing(Box::new(err)),
            })
        }
    };
    Ok(Type::VarArgs(value_type.map(Box::new)))
}

fn parse_list(iter: &mut std::str::Chars) -> Result<Type, ParseError> {
    let list_str = iter.as_str();
    advance_once(iter.by_ref());
    let value_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                ParseError::UnexpectedChar(CHAR_LIST_END, _) | ParseError::EndOfInput => {
                    ParseError::MissingListValueType(list_str.to_owned())
                }
                _ => ParseError::ListValueTypeParsing(Box::new(err)),
            })
        }
    };
    if iter.clone().next() != Some(CHAR_LIST_END) {
        return Err(ParseError::MissingListEnd(list_str.to_owned()));
    }
    advance_once(iter);
    Ok(Type::List(value_type.map(Box::new)))
}

fn parse_map(iter: &mut std::str::Chars) -> Result<Type, ParseError> {
    let map_str = iter.as_str();
    advance_once(iter.by_ref());
    let key_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                ParseError::UnexpectedChar(CHAR_MAP_END, _) | ParseError::EndOfInput => {
                    ParseError::MissingMapKeyType(map_str.to_owned())
                }
                _ => ParseError::MapKeyTypeParsing(Box::new(err)),
            })
        }
    };
    let value_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                ParseError::UnexpectedChar(CHAR_MAP_END, _) => {
                    ParseError::MissingMapValueType(map_str.to_owned())
                }
                _ => ParseError::MapValueTypeParsing(Box::new(err)),
            })
        }
    };
    if iter.clone().next() != Some(CHAR_MAP_END) {
        return Err(ParseError::MissingMapEnd(map_str.to_owned()));
    }
    advance_once(iter.by_ref());
    Ok(Type::Map {
        key: key_type.map(Box::new),
        value: value_type.map(Box::new),
    })
}

fn parse_tuple(iter: &mut std::str::Chars) -> Result<Type, ParseError> {
    let tuple_str = iter.as_str();
    advance_once(iter.by_ref());
    let mut elements = Vec::new();
    let elements = loop {
        match parse_type(iter) {
            Ok(element) => elements.push(element),
            Err(err) => match err {
                ParseError::UnexpectedChar(CHAR_TUPLE_END, _) => break elements,
                ParseError::EndOfInput => {
                    return Err(ParseError::MissingTupleEnd(tuple_str.to_owned()))
                }
                _ => return Err(ParseError::TupleElementTypeParsing(Box::new(err))),
            },
        }
    };

    let tuple = {
        match iter.clone().next() {
            Some(CHAR_ANNOTATIONS_BEGIN) => {
                let annotations_str = iter.as_str();
                let annotations = match parse_tuple_annotations(iter) {
                    Ok(annotations) => annotations,
                    Err(err) => {
                        return Err(ParseError::Annotations {
                            annotations: annotations_str.to_owned(),
                            tuple: tuple_str.to_owned(),
                            source: err,
                        })
                    }
                };
                let tuple = match annotations {
                    Some(annotations) => {
                        Tuple::struct_from_annotations_of_elements(annotations, elements).map_err(
                            |err| ParseError::Annotations {
                                annotations: annotations_str.to_owned(),
                                tuple: tuple_str.to_owned(),
                                source: err.into(),
                            },
                        )?
                    }
                    None => Tuple::Tuple(elements),
                };
                Type::Tuple(tuple)
            }
            _ => Type::Tuple(Tuple::Tuple(elements)),
        }
    };

    Ok(tuple)
}

fn parse_tuple_annotations(
    iter: &mut std::str::Chars,
) -> Result<Option<StructAnnotations>, AnnotationsError> {
    advance_once(iter.by_ref());
    enum Accumulator {
        Name(Option<String>),
        Field {
            name: Option<String>,
            previous_fields: Option<Vec<String>>,
            current: Option<String>,
        },
    }
    impl Accumulator {
        fn new() -> Self {
            Self::Name(None)
        }
        fn push_char(&mut self, c: char) {
            match self {
                Self::Name(s) | Self::Field { current: s, .. } => match s {
                    Some(s) => s.push(c),
                    None => *s = Some(String::from(c)),
                },
            }
        }
        fn next(self) -> Self {
            match self {
                Self::Name(name) => Self::Field {
                    name,
                    previous_fields: None,
                    current: None,
                },
                Self::Field {
                    name,
                    mut previous_fields,
                    current,
                } => {
                    if let Some(field) = current {
                        previous_fields.get_or_insert_with(Vec::new).push(field)
                    }
                    Self::Field {
                        name,
                        previous_fields,
                        current: None,
                    }
                }
            }
        }

        fn end(self) -> Option<StructAnnotations> {
            match self {
                Self::Name(None) | Self::Field { name: None, .. } => None,
                Self::Name(Some(name)) => Some(StructAnnotations {
                    name,
                    field_names: None,
                }),
                Self::Field {
                    name: Some(name),
                    previous_fields: mut fields,
                    current,
                } => Some({
                    if let Some(field) = current {
                        fields.get_or_insert_with(Vec::new).push(field);
                    }
                    StructAnnotations {
                        name,
                        field_names: fields,
                    }
                }),
            }
        }
    }
    let annotations = {
        let mut accu = Accumulator::new();
        loop {
            match iter.next() {
                Some(CHAR_ANNOTATIONS_SEP) => accu = accu.next(),
                Some(CHAR_ANNOTATIONS_END) => break accu.end(),
                Some(c) if c.is_ascii() && (c.is_alphanumeric() || c == '_') => accu.push_char(c),
                Some(' ') => { /* spaces are ignored */ }
                Some(c) => return Err(AnnotationsError::UnexpectedChar(c)),
                None => return Err(AnnotationsError::MissingTupleAnnotationEnd),
            }
        }
    };
    Ok(annotations)
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ParseError {
    #[error("end of input reached")]
    EndOfInput,

    #[error("unexpected character \'{0}\' in input \"{1}\"")]
    UnexpectedChar(char, String),

    #[error("value type of option starting at input \"{0}\" is missing")]
    MissingOptionValueType(String),

    #[error("parsing of option value type failed")]
    OptionValueTypeParsing(#[source] Box<ParseError>),

    #[error("value type of varargs starting at input \"{0}\" is missing")]
    MissingVarArgsValueType(String),

    #[error("parsing of varargs value type failed")]
    VarArgsValueTypeParsing(#[source] Box<ParseError>),

    #[error("value type of list starting at input \"{0}\" is missing")]
    MissingListValueType(String),

    #[error("parsing of list value type failed")]
    ListValueTypeParsing(#[source] Box<ParseError>),

    #[error("end of list starting at input \"{0}\" is missing")]
    MissingListEnd(String),

    #[error("key type of map starting at input \"{0}\" is missing")]
    MissingMapKeyType(String),

    #[error("parsing of map key type failed")]
    MapKeyTypeParsing(#[source] Box<ParseError>),

    #[error("value type of map starting at input \"{0}\" is missing")]
    MissingMapValueType(String),

    #[error("parsing of map value type failed")]
    MapValueTypeParsing(#[source] Box<ParseError>),

    #[error("end of map starting at input \"{0}\" is missing")]
    MissingMapEnd(String),

    #[error("parsing of a tuple element type failed")]
    TupleElementTypeParsing(#[source] Box<ParseError>),

    #[error("end of tuple starting at input \"{0}\" is missing")]
    MissingTupleEnd(String),

    #[error("parsing of tuple \"{tuple}\" annotations starting at input \"{annotations}\" failed")]
    Annotations {
        annotations: String,
        tuple: String,
        source: AnnotationsError,
    },
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum AnnotationsError {
    #[error("end of annotations is missing")]
    MissingTupleAnnotationEnd,

    #[error("unexpected annotations character '{0}'")]
    UnexpectedChar(char),

    #[error(transparent)]
    ZipError(#[from] ty::ZipStructFieldsSizeError),
}
