use crate::ty::{self, StructAnnotations, TupleType, Type};
use derive_more::{From, Into};

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, From, Into)]
#[into(owned, ref, ref_mut)]
pub struct Signature(Option<Type>);

impl Signature {
    pub fn new(t: Option<Type>) -> Self {
        Self(t)
    }

    pub fn dynamic() -> Self {
        Self(None)
    }

    pub fn into_type(self) -> Option<Type> {
        self.0
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_type(self.0.as_ref(), f)
    }
}

impl std::str::FromStr for Signature {
    type Err = FromStrError;

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

fn parse_type(iter: &mut std::str::Chars) -> Result<Option<Type>, SignatureParseError> {
    let type_str = iter.as_str();
    // Multiple characters types are read from the beginning. Therefore we clone the iterator,
    // read one char, and if we detect any marker of those types, pass the original iterator to
    // the subparsing function and return its result immediately.
    let c = iter.clone().next().ok_or(SignatureParseError::EndOfInput)?;
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
        c => return Err(SignatureParseError::UnexpectedChar(c, type_str.to_owned())),
    };
    Ok(t)
}

fn parse_option(iter: &mut std::str::Chars) -> Result<Type, SignatureParseError> {
    let option_str = iter.as_str();
    advance_once(iter.by_ref());
    let value_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                SignatureParseError::EndOfInput => {
                    SignatureParseError::MissingOptionValueType(option_str.to_owned())
                }
                _ => SignatureParseError::OptionValueTypeParsing(Box::new(err)),
            })
        }
    };
    Ok(Type::Option(value_type.map(Box::new)))
}

fn parse_var_args(iter: &mut std::str::Chars) -> Result<Type, SignatureParseError> {
    let var_args_str = iter.as_str();
    advance_once(iter.by_ref());
    let value_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                SignatureParseError::EndOfInput => {
                    SignatureParseError::MissingVarArgsValueType(var_args_str.to_owned())
                }
                _ => SignatureParseError::VarArgsValueTypeParsing(Box::new(err)),
            })
        }
    };
    Ok(Type::VarArgs(value_type.map(Box::new)))
}

fn parse_list(iter: &mut std::str::Chars) -> Result<Type, SignatureParseError> {
    let list_str = iter.as_str();
    advance_once(iter.by_ref());
    let value_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                SignatureParseError::UnexpectedChar(CHAR_LIST_END, _)
                | SignatureParseError::EndOfInput => {
                    SignatureParseError::MissingListValueType(list_str.to_owned())
                }
                _ => SignatureParseError::ListValueTypeParsing(Box::new(err)),
            })
        }
    };
    if iter.clone().next() != Some(CHAR_LIST_END) {
        return Err(SignatureParseError::MissingListEnd(list_str.to_owned()));
    }
    advance_once(iter);
    Ok(Type::List(value_type.map(Box::new)))
}

fn parse_map(iter: &mut std::str::Chars) -> Result<Type, SignatureParseError> {
    let map_str = iter.as_str();
    advance_once(iter.by_ref());
    let key_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                SignatureParseError::UnexpectedChar(CHAR_MAP_END, _)
                | SignatureParseError::EndOfInput => {
                    SignatureParseError::MissingMapKeyType(map_str.to_owned())
                }
                _ => SignatureParseError::MapKeyTypeParsing(Box::new(err)),
            })
        }
    };
    let value_type = match parse_type(iter) {
        Ok(t) => t,
        Err(err) => {
            return Err(match err {
                SignatureParseError::UnexpectedChar(CHAR_MAP_END, _) => {
                    SignatureParseError::MissingMapValueType(map_str.to_owned())
                }
                _ => SignatureParseError::MapValueTypeParsing(Box::new(err)),
            })
        }
    };
    if iter.clone().next() != Some(CHAR_MAP_END) {
        return Err(SignatureParseError::MissingMapEnd(map_str.to_owned()));
    }
    advance_once(iter.by_ref());
    Ok(Type::Map {
        key: key_type.map(Box::new),
        value: value_type.map(Box::new),
    })
}

fn parse_tuple(iter: &mut std::str::Chars) -> Result<Type, SignatureParseError> {
    let tuple_str = iter.as_str();
    advance_once(iter.by_ref());
    let mut elements = Vec::new();
    let elements = loop {
        match parse_type(iter) {
            Ok(element) => elements.push(element),
            Err(err) => match err {
                SignatureParseError::UnexpectedChar(CHAR_TUPLE_END, _) => break elements,
                SignatureParseError::EndOfInput => {
                    return Err(SignatureParseError::MissingTupleEnd(tuple_str.to_owned()))
                }
                _ => return Err(SignatureParseError::TupleElementTypeParsing(Box::new(err))),
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
                        return Err(SignatureParseError::Annotations {
                            annotations: annotations_str.to_owned(),
                            tuple: tuple_str.to_owned(),
                            source: err,
                        })
                    }
                };
                let tuple = match annotations {
                    Some(annotations) => {
                        TupleType::from_annotations_of_elements(annotations, elements).map_err(
                            |err| SignatureParseError::Annotations {
                                annotations: annotations_str.to_owned(),
                                tuple: tuple_str.to_owned(),
                                source: err.into(),
                            },
                        )?
                    }
                    None => TupleType::Tuple(elements),
                };
                Type::Tuple(tuple)
            }
            _ => Type::Tuple(TupleType::Tuple(elements)),
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
                Some(c) if c == ' ' => { /* spaces are ignored */ }
                Some(c) => return Err(AnnotationsError::UnexpectedChar(c)),
                None => return Err(AnnotationsError::MissingTupleAnnotationEnd),
            }
        }
    };
    Ok(annotations)
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
#[error("error while parsing signature: {0}")]
pub struct FromStrError(#[from] pub SignatureParseError);

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum SignatureParseError {
    #[error("end of input reached")]
    EndOfInput,

    #[error("unexpected character \'{0}\' in input \"{1}\"")]
    UnexpectedChar(char, String),

    #[error("value type of option starting at input \"{0}\" is missing")]
    MissingOptionValueType(String),

    #[error("parsing of option value type failed")]
    OptionValueTypeParsing(#[source] Box<SignatureParseError>),

    #[error("value type of varargs starting at input \"{0}\" is missing")]
    MissingVarArgsValueType(String),

    #[error("parsing of varargs value type failed")]
    VarArgsValueTypeParsing(#[source] Box<SignatureParseError>),

    #[error("value type of list starting at input \"{0}\" is missing")]
    MissingListValueType(String),

    #[error("parsing of list value type failed")]
    ListValueTypeParsing(#[source] Box<SignatureParseError>),

    #[error("end of list starting at input \"{0}\" is missing")]
    MissingListEnd(String),

    #[error("key type of map starting at input \"{0}\" is missing")]
    MissingMapKeyType(String),

    #[error("parsing of map key type failed")]
    MapKeyTypeParsing(#[source] Box<SignatureParseError>),

    #[error("value type of map starting at input \"{0}\" is missing")]
    MissingMapValueType(String),

    #[error("parsing of map value type failed")]
    MapValueTypeParsing(#[source] Box<SignatureParseError>),

    #[error("end of map starting at input \"{0}\" is missing")]
    MissingMapEnd(String),

    #[error("parsing of a tuple element type failed")]
    TupleElementTypeParsing(#[source] Box<SignatureParseError>),

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

impl serde::Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Signature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a borrowed or owned string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(serde::de::Error::custom)
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{list_ty, map_ty, option_ty, struct_ty, tuple_ty, varargs_ty, MetaObject};

    #[test]
    fn test_signature_to_from_string() {
        use pretty_assertions::assert_eq;
        macro_rules! assert_sig_to_str {
            ($t:expr, $s:expr) => {{
                assert_eq!(
                    Signature($t.into()).to_string(),
                    $s,
                    "signature of ({t:?}) is not {s:?}",
                    t = $t,
                    s = $s
                );
            }};
        }
        macro_rules! assert_sig_from_str {
            ($t:expr, $s:expr) => {{
                assert_eq!(
                    $s.parse::<Signature>().map(|s| s.into_type()),
                    Ok($t.into()),
                    "{s:?} into a signature is not {t:?}",
                    s = $s,
                    t = $t
                );
            }};
        }
        macro_rules! assert_sig_from_to_str {
            ($t:expr, $s:expr) => {{
                assert_sig_from_to_str!($s => $t => $s);
            }};
            ($from_s:expr => $t:expr => $to_s:expr) => {{
                assert_sig_from_str!($t, $from_s);
                assert_sig_to_str!($t, $to_s);
            }};
        }
        assert_sig_from_to_str!(Type::Unit, "v");
        assert_sig_from_to_str!(Type::Bool, "b");
        assert_sig_from_to_str!(Type::Int8, "c");
        assert_sig_from_to_str!(Type::UInt8, "C");
        assert_sig_from_to_str!(Type::Int16, "w");
        assert_sig_from_to_str!(Type::UInt16, "W");
        assert_sig_from_to_str!(Type::Int32, "i");
        assert_sig_from_to_str!(Type::UInt32, "I");
        assert_sig_from_to_str!(Type::Int64, "l");
        assert_sig_from_to_str!(Type::UInt64, "L");
        assert_sig_from_to_str!(Type::Float32, "f");
        assert_sig_from_to_str!(Type::Float64, "d");
        assert_sig_from_to_str!(Type::String, "s");
        assert_sig_from_to_str!(Type::Raw, "r");
        assert_sig_from_to_str!(Type::Object, "o");
        assert_sig_from_to_str!(None::<Type>, "m");
        assert_sig_from_to_str!(ty::option_of(Type::Unit), "+v");
        assert_sig_from_to_str!(ty::varargs_of(None), "#m");
        assert_sig_from_to_str!(ty::list_of(Type::Int32), "[i]");
        assert_sig_from_to_str!(ty::list_of(tuple_ty![]), "[()]");
        assert_sig_from_to_str!(ty::map_of(Type::Float32, Type::String), "{fs}");
        assert_sig_from_to_str!(
            tuple_ty![Type::Float32, Type::String, Type::UInt32],
            "(fsI)"
        );
        assert_sig_from_to_str!(
            struct_ty! {
                ExplorationMap(
                    list_ty!(tuple_ty!(Type::Float64, Type::Float64)),
                    Type::UInt64
                )
            },
            "([(dd)]L)<ExplorationMap>"
        );
        assert_sig_from_to_str!(
            struct_ty! {
                ExplorationMap {
                    points: list_ty!(struct_ty! {
                        Point {
                            x: Type::Float64,
                            y: Type::Float64,
                        }
                    }),
                    timestamp: Type::UInt64,
                }
            },
            "([(dd)<Point,x,y>]L)<ExplorationMap,points,timestamp>"
        );
        // Underscores in structure and field names are allowed.
        // Spaces between structure or field names are trimmed.
        assert_sig_from_to_str!(
            "(i)<   A_B ,  c_d   >" =>
            struct_ty!{
                A_B {
                    c_d: Type::Int32
                }
            } =>
            "(i)<A_B,c_d>"
        );
        // Annotations can be ignored if the struct name is missing.
        assert_sig_from_to_str!("()<>" => tuple_ty!() => "()");
        assert_sig_from_to_str!("(i)<>" => tuple_ty!(Type::Int32) => "(i)");
        assert_sig_from_to_str!("(i)<,,,,,,,>" => tuple_ty!(Type::Int32) => "(i)");
        assert_sig_from_to_str!("(ff)<,x,y>" => tuple_ty!(Type::Float32, Type::Float32) => "(ff)");
        // Some complex type for fun.
        assert_sig_from_to_str!(
            tuple_ty!(
                list_ty!(map_ty!(option_ty!(Type::Object), Type::Raw)),
                varargs_ty!(option_ty!(None))
            ),
            "([{+or}]#+m)"
        );
    }

    #[test]
    fn test_signature_from_str_errors() {
        assert_eq!(
            "".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::EndOfInput))
        );
        assert_eq!(
            "u".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::UnexpectedChar(
                'u',
                "u".to_owned()
            )))
        );
        // Option
        assert_eq!(
            "+".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingOptionValueType(
                "+".to_owned()
            )))
        );
        assert_eq!(
            "+[".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::OptionValueTypeParsing(
                Box::new(SignatureParseError::MissingListValueType("[".to_owned()))
            )))
        );
        // VarArgs
        assert_eq!(
            "#".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingVarArgsValueType(
                "#".to_owned()
            )))
        );
        assert_eq!(
            "#[".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::VarArgsValueTypeParsing(
                Box::new(SignatureParseError::MissingListValueType("[".to_owned()))
            )))
        );
        // Lists
        assert_eq!(
            "[".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingListValueType(
                "[".to_owned()
            )))
        );
        assert_eq!(
            "[]".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingListValueType(
                "[]".to_owned()
            )))
        );
        assert_eq!(
            "[i".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingListEnd(
                "[i".to_owned()
            )))
        );
        assert_eq!(
            "[{i}]".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::ListValueTypeParsing(
                Box::new(SignatureParseError::MissingMapValueType("{i}]".to_owned()))
            )))
        );
        // The error is `UnexpectedChar` and not `MissingTupleEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "[(]".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::ListValueTypeParsing(
                Box::new(SignatureParseError::TupleElementTypeParsing(Box::new(
                    SignatureParseError::UnexpectedChar(']', "]".to_owned())
                )))
            )))
        );
        // Maps
        assert_eq!(
            "{".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingMapKeyType(
                "{".to_owned()
            )))
        );
        assert_eq!(
            "{}".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingMapKeyType(
                "{}".to_owned()
            )))
        );
        assert_eq!(
            "{i}".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingMapValueType(
                "{i}".to_owned()
            )))
        );
        assert_eq!(
            "{ii".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingMapEnd(
                "{ii".to_owned()
            )))
        );
        assert_eq!(
            "{[]i}".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MapKeyTypeParsing(
                Box::new(SignatureParseError::MissingListValueType("[]i}".to_owned()))
            )))
        );
        assert_eq!(
            "{i[]}".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MapValueTypeParsing(
                Box::new(SignatureParseError::MissingListValueType("[]}".to_owned()))
            )))
        );
        // The error is `UnexpectedChar` and not `MissingListEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "{i[}".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MapValueTypeParsing(
                Box::new(SignatureParseError::ListValueTypeParsing(Box::new(
                    SignatureParseError::UnexpectedChar('}', "}".to_owned())
                )))
            )))
        );
        // Tuples
        assert_eq!(
            "(".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingTupleEnd(
                "(".to_owned()
            )))
        );
        assert_eq!(
            "(iii".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::MissingTupleEnd(
                "(iii".to_owned()
            )))
        );
        assert_eq!(
            "(i[i)".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::TupleElementTypeParsing(
                Box::new(SignatureParseError::MissingListEnd("[i)".to_owned()))
            )))
        );
        // Tuples annotations
        assert_eq!(
            "(i)<".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::Annotations {
                annotations: "<".to_owned(),
                tuple: "(i)<".to_owned(),
                source: AnnotationsError::MissingTupleAnnotationEnd
            }))
        );
        assert_eq!(
            "(i)<S,a,b>".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::Annotations {
                annotations: "<S,a,b>".to_owned(),
                tuple: "(i)<S,a,b>".to_owned(),
                source: AnnotationsError::ZipError(ty::ZipStructFieldsSizeError {
                    name_count: 2,
                    element_count: 1
                }),
            }))
        );
        //   - Only ASCII is supported
        assert_eq!(
            "(i)<越>".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::Annotations {
                annotations: "<越>".to_owned(),
                tuple: "(i)<越>".to_owned(),
                source: AnnotationsError::UnexpectedChar('越'),
            }))
        );

        // The error is `UnexpectedChar` and not `MissingMapEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "(i{i)".parse::<Signature>(),
            Err(FromStrError(SignatureParseError::TupleElementTypeParsing(
                Box::new(SignatureParseError::MapValueTypeParsing(Box::new(
                    SignatureParseError::UnexpectedChar(')', ")".to_owned())
                )))
            )))
        );
    }

    #[test]
    fn test_signature_from_str_meta_object() {
        use pretty_assertions::assert_eq;
        let input = "({I(Issss[(ss)<MetaMethodParameter,name,description>]s)\
                     <MetaMethod,uid,returnSignature,name,parametersSignature,\
                     description,parameters,returnDescription>}{I(Iss)<MetaSignal,\
                     uid,name,signature>}{I(Iss)<MetaProperty,uid,name,signature>}s)\
                     <MetaObject,methods,signals,properties,description>";
        let sig: Signature = input.parse().unwrap();
        use ty::StaticGetType;
        assert_eq!(sig, Signature(Some(MetaObject::ty())));
    }

    #[test]
    fn test_signature_ser_de() {
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &Signature(Some(struct_ty! {
                Point {
                    x: Type::Float64,
                    y: Type::Float64,
                }
            })),
            &[Token::Str("(dd)<Point,x,y>")],
        )
    }
}
