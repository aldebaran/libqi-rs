use super::r#type::Type;

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Signature(Type);

impl Signature {
    pub fn into_type(self) -> Type {
        self.0
    }

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
    const CHAR_MARK_VAR_ARGS: char = '#';
    const CHAR_MARK_KW_ARGS: char = '~';
    const CHAR_ANNOTATIONS_BEGIN: char = '<';
    const CHAR_ANNOTATIONS_SEP: char = ',';
    const CHAR_ANNOTATIONS_END: char = '>';

    fn parse_type<'c>(
        iter: &std::str::Chars<'c>,
    ) -> Result<(Type, std::str::Chars<'c>), FromStrError> {
        let mut iter_after_value = iter.clone();
        let value = match iter_after_value.next().ok_or(FromStrError::EndOfInput)? {
            Self::CHAR_NONE => Type::None,
            Self::CHAR_UNKNOWN => Type::Unknown,
            Self::CHAR_VOID => Type::Void,
            Self::CHAR_BOOL => Type::Bool,
            Self::CHAR_INT8 => Type::Int8,
            Self::CHAR_UINT8 => Type::UInt8,
            Self::CHAR_INT16 => Type::Int16,
            Self::CHAR_UINT16 => Type::UInt16,
            Self::CHAR_INT32 => Type::Int32,
            Self::CHAR_UINT32 => Type::UInt32,
            Self::CHAR_INT64 => Type::Int64,
            Self::CHAR_UINT64 => Type::UInt64,
            Self::CHAR_FLOAT => Type::Float,
            Self::CHAR_DOUBLE => Type::Double,
            Self::CHAR_STRING => Type::String,
            Self::CHAR_RAW => Type::Raw,
            Self::CHAR_OBJECT => Type::Object,
            Self::CHAR_DYNAMIC => Type::Dynamic,
            c => {
                let (value, iter_end) = match c {
                    Self::CHAR_MARK_OPTION => Self::parse_option(iter),
                    Self::CHAR_LIST_BEGIN => Self::parse_list(iter),
                    Self::CHAR_MAP_BEGIN => Self::parse_map(iter),
                    Self::CHAR_TUPLE_BEGIN => Self::parse_tuple(iter),
                    Self::CHAR_MARK_VAR_ARGS => Self::parse_var_args(iter),
                    Self::CHAR_MARK_KW_ARGS => Self::parse_kw_args(iter),
                    c => return Err(FromStrError::UnexpectedChar(c, iter.as_str().into())),
                }?;
                iter_after_value = iter_end;
                value
            }
        };
        Ok((value, iter_after_value))
    }

    fn pass_marker<'c>(iter: &std::str::Chars<'c>) -> std::str::Chars<'c> {
        let mut iter = iter.clone();
        match iter.next() {
            Some(_) => iter,
            None => unreachable!(),
        }
    }

    fn parse_option<'c>(
        iter: &std::str::Chars<'c>,
    ) -> Result<(Type, std::str::Chars<'c>), FromStrError> {
        let iter_value = Self::pass_marker(iter);
        let (value, iter_after_value) = match Self::parse_type(&iter_value) {
            Ok(t) => t,
            Err(err) => {
                return Err(match err {
                    FromStrError::EndOfInput => {
                        FromStrError::MissingOptionValueType(iter.as_str().into())
                    }
                    _ => FromStrError::OptionValueTypeParsing(Box::new(err)),
                })
            }
        };
        Ok((Type::Option(value.into()), iter_after_value))
    }

    fn parse_list<'c>(
        iter: &std::str::Chars<'c>,
    ) -> Result<(Type, std::str::Chars<'c>), FromStrError> {
        let iter_value = Self::pass_marker(iter);
        let (value, mut iter_after_value) = match Self::parse_type(&iter_value) {
            Ok(t) => t,
            Err(err) => {
                return Err(match err {
                    FromStrError::UnexpectedChar(Self::CHAR_LIST_END, _)
                    | FromStrError::EndOfInput => {
                        FromStrError::MissingListValueType(iter.as_str().into())
                    }
                    _ => FromStrError::ListValueTypeParsing(Box::new(err)),
                })
            }
        };
        if iter_after_value.next() != Some(Self::CHAR_LIST_END) {
            return Err(FromStrError::MissingListEnd(iter.as_str().into()));
        }
        Ok((Type::List(value.into()), iter_after_value))
    }

    fn parse_map<'c>(
        iter: &std::str::Chars<'c>,
    ) -> Result<(Type, std::str::Chars<'c>), FromStrError> {
        let iter_value = Self::pass_marker(iter);
        let (key, iter_key) = match Self::parse_type(&iter_value) {
            Ok(t) => t,
            Err(err) => {
                return Err(match err {
                    FromStrError::UnexpectedChar(Self::CHAR_MAP_END, _)
                    | FromStrError::EndOfInput => {
                        FromStrError::MissingMapKeyType(iter.as_str().into())
                    }
                    _ => FromStrError::MapKeyTypeParsing(Box::new(err)),
                })
            }
        };
        let (value, mut iter_after_value) = match Self::parse_type(&iter_key) {
            Ok(t) => t,
            Err(err) => {
                return Err(match err {
                    FromStrError::UnexpectedChar(Self::CHAR_MAP_END, _) => {
                        FromStrError::MissingMapValueType(iter.as_str().into())
                    }
                    _ => FromStrError::MapValueTypeParsing(Box::new(err)),
                })
            }
        };
        if iter_after_value.next() != Some(Self::CHAR_MAP_END) {
            return Err(FromStrError::MissingMapEnd(iter.as_str().into()));
        }
        Ok((
            Type::Map {
                key: key.into(),
                value: value.into(),
            },
            iter_after_value,
        ))
    }

    fn parse_var_args<'c>(
        iter: &std::str::Chars<'c>,
    ) -> Result<(Type, std::str::Chars<'c>), FromStrError> {
        let iter_value = Self::pass_marker(iter);
        let (value_type, iter_after_value) = match Self::parse_type(&iter_value) {
            Ok(t) => t,
            Err(err) => {
                return Err(match err {
                    FromStrError::EndOfInput => {
                        FromStrError::MissingVarArgsValueType(iter.as_str().into())
                    }
                    _ => FromStrError::VarArgsValueTypeParsing(Box::new(err)),
                })
            }
        };
        Ok((Type::VarArgs(value_type.into()), iter_after_value))
    }

    fn parse_kw_args<'c>(
        iter: &std::str::Chars<'c>,
    ) -> Result<(Type, std::str::Chars<'c>), FromStrError> {
        let iter_value = Self::pass_marker(iter);
        let (value_type, iter_after_value) = match Self::parse_type(&iter_value) {
            Ok(t) => t,
            Err(err) => {
                return Err(match err {
                    FromStrError::EndOfInput => {
                        FromStrError::MissingKwArgsValueType(iter.as_str().into())
                    }
                    _ => FromStrError::KwArgsValueTypeParsing(Box::new(err)),
                })
            }
        };
        Ok((Type::KwArgs(value_type.into()), iter_after_value))
    }

    fn parse_tuple<'c>(
        iter: &std::str::Chars<'c>,
    ) -> Result<(Type, std::str::Chars<'c>), FromStrError> {
        let mut iter_elements = Self::pass_marker(iter);
        let mut elements = Vec::new();
        let elements = loop {
            match Self::parse_type(&iter_elements) {
                Ok((element, iter_after_element)) => {
                    elements.push(element);
                    iter_elements = iter_after_element;
                }
                Err(err) => match err {
                    FromStrError::UnexpectedChar(Self::CHAR_TUPLE_END, _) => break elements,
                    FromStrError::EndOfInput => {
                        return Err(FromStrError::MissingTupleEnd(iter.as_str().into()))
                    }
                    _ => return Err(FromStrError::TupleElementTypeParsing(Box::new(err))),
                },
            }
        };
        let mut iter_after_tuple = iter_elements.clone();

        let tuple = {
            let iter_annotations = iter_elements;
            match iter_annotations.clone().next() {
                Some(Signature::CHAR_ANNOTATIONS_BEGIN) => {
                    match Self::parse_annotations(&iter_annotations, iter) {
                        Ok((Annotations { name, field_names }, iter_after_annotations)) => {
                            iter_after_tuple = iter_after_annotations;
                            match (name, field_names) {
                                (Some(name), Some(field_names)) => {
                                    if field_names.len() != elements.len() {
                                        return Err(FromStrError::Annotation(
                                            iter.as_str().into(),
                                            format!("expected {elems_len} annotations but got {names_len}",
                                                    elems_len=elements.len(), names_len = field_names.len()),
                                        ));
                                    }
                                    let fields = field_names.into_iter().zip(elements).collect();
                                    Type::Struct { name, fields }
                                }
                                (Some(name), None) => Type::TupleStruct { name, elements },
                                (_, _) => Type::Tuple(elements),
                            }
                        }
                        Err(err) => {
                            return Err(FromStrError::Annotation(
                                iter.as_str().into(),
                                err.to_string(),
                            ))
                        }
                    }
                }
                _ => Type::Tuple(elements),
            }
        };

        Ok((tuple, iter_after_tuple))
    }

    fn parse_annotations<'c>(
        iter: &std::str::Chars<'c>,
        iter_tuple: &std::str::Chars<'c>,
    ) -> Result<(Annotations, std::str::Chars<'c>), FromStrError> {
        let mut iter_annotations = Self::pass_marker(iter);
        enum State {
            Name(Option<String>),
            Field(Option<String>),
        }
        impl State {
            fn push_char(&mut self, c: char) {
                match self {
                    Self::Name(s) | Self::Field(s) => match s {
                        Some(s) => s.push(c),
                        None => *s = Some(String::from(c)),
                    },
                }
            }
            fn next(&mut self, annotations: &mut Annotations) {
                match std::mem::replace(self, Self::Field(None)) {
                    Self::Name(name) => {
                        annotations.name = name;
                    }
                    Self::Field(field) => {
                        if let Some(f) = field {
                            let fields = &mut annotations.field_names;
                            let fields = fields.get_or_insert_with(Vec::new);
                            fields.push(f);
                        }
                    }
                }
            }
        }
        let mut annotations = Annotations {
            name: None,
            field_names: None,
        };
        let mut state = State::Name(None);
        let value = loop {
            match iter_annotations.next() {
                Some(Self::CHAR_ANNOTATIONS_SEP) => state.next(&mut annotations),
                Some(Self::CHAR_ANNOTATIONS_END) => {
                    state.next(&mut annotations);
                    break annotations;
                }
                Some(c) if c.is_alphanumeric() => state.push_char(c),
                Some(c) => return Err(FromStrError::UnexpectedChar(c, iter_tuple.as_str().into())),
                None => {
                    return Err(FromStrError::MissingTupleAnnotationEnd(
                        iter.as_str().into(),
                        iter_tuple.as_str().into(),
                    ))
                }
            }
        };
        Ok((value, iter_annotations))
    }
}

struct Annotations {
    name: Option<String>,
    field_names: Option<Vec<String>>,
}

impl From<Type> for Signature {
    fn from(t: Type) -> Self {
        Self(t)
    }
}

impl From<Signature> for Type {
    fn from(s: Signature) -> Self {
        s.0
    }
}

fn write_type(t: &Type, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use std::fmt::Write;
    match t {
        Type::None => f.write_char(Signature::CHAR_NONE),
        Type::Unknown => f.write_char(Signature::CHAR_UNKNOWN),
        Type::Void => f.write_char(Signature::CHAR_VOID),
        Type::Bool => f.write_char(Signature::CHAR_BOOL),
        Type::Int8 => f.write_char(Signature::CHAR_INT8),
        Type::UInt8 => f.write_char(Signature::CHAR_UINT8),
        Type::Int16 => f.write_char(Signature::CHAR_INT16),
        Type::UInt16 => f.write_char(Signature::CHAR_UINT16),
        Type::Int32 => f.write_char(Signature::CHAR_INT32),
        Type::UInt32 => f.write_char(Signature::CHAR_UINT32),
        Type::Int64 => f.write_char(Signature::CHAR_INT64),
        Type::UInt64 => f.write_char(Signature::CHAR_UINT64),
        Type::Float => f.write_char(Signature::CHAR_FLOAT),
        Type::Double => f.write_char(Signature::CHAR_DOUBLE),
        Type::String => f.write_char(Signature::CHAR_STRING),
        Type::Raw => f.write_char(Signature::CHAR_RAW),
        Type::Object => f.write_char(Signature::CHAR_OBJECT),
        Type::Dynamic => f.write_char(Signature::CHAR_DYNAMIC),
        Type::Option(o) => {
            f.write_char(Signature::CHAR_MARK_OPTION)?;
            write_type(o.as_ref(), f)
        }
        Type::List(t) => {
            f.write_char(Signature::CHAR_LIST_BEGIN)?;
            write_type(t.as_ref(), f)?;
            f.write_char(Signature::CHAR_LIST_END)
        }
        Type::Map { key, value } => {
            f.write_char(Signature::CHAR_MAP_BEGIN)?;
            write_type(key.as_ref(), f)?;
            write_type(value.as_ref(), f)?;
            f.write_char(Signature::CHAR_MAP_END)
        }
        Type::Tuple(elements) => {
            f.write_char(Signature::CHAR_TUPLE_BEGIN)?;
            for element in elements {
                write_type(element, f)?;
            }
            f.write_char(Signature::CHAR_TUPLE_END)
        }
        Type::TupleStruct { .. } => todo!(),
        Type::Struct { name, fields } => {
            f.write_char(Signature::CHAR_TUPLE_BEGIN)?;
            for (_, field) in fields {
                write_type(field, f)?;
            }
            f.write_char(Signature::CHAR_TUPLE_END)?;
            f.write_char(Signature::CHAR_ANNOTATIONS_BEGIN)?;
            f.write_str(name)?;
            for (name, _) in fields {
                write!(f, ",{name}")?;
            }
            f.write_char(Signature::CHAR_ANNOTATIONS_END)
        }
        Type::VarArgs(t) => {
            f.write_char(Signature::CHAR_MARK_VAR_ARGS)?;
            write_type(t, f)
        }
        Type::KwArgs(t) => {
            f.write_char(Signature::CHAR_MARK_KW_ARGS)?;
            write_type(t, f)
        }
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_type(&self.0, f)
    }
}

impl std::str::FromStr for Signature {
    type Err = FromStrError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match Self::parse_type(&src.chars()) {
            Ok((t, _)) => Ok(Self(t)),
            Err(err) => Err(err),
        }
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

    #[error("annotation of structure starting at input \"{0}\" failed: {1}")]
    Annotation(String, String),

    #[error("end of annotations starting at input \"{0}\" for tuple starting at input \"{1}\" is missing")]
    MissingTupleAnnotationEnd(String, String),
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
                v.parse().map_err(|e| serde::de::Error::custom(e))
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn test_signature_to_from_string() {
        use pretty_assertions::assert_eq;
        macro_rules! assert_sig_to_str {
            ($t:expr, $s:expr) => {{
                assert_eq!(
                    Signature($t).to_string(),
                    $s,
                    "Left is {t:?}.to_string(), Right is {s:?}",
                    t = $t,
                    s = $s
                );
            }};
        }
        macro_rules! assert_sig_from_str {
            ($t:expr, $s:expr) => {{
                assert_eq!(
                    $s.parse::<Signature>().map(|s| s.into_type()),
                    Ok($t),
                    "Left is {s:?}.parse(), Right is {t:?}",
                    s = $s,
                    t = $t
                );
            }};
        }
        macro_rules! assert_sig_to_from_str {
            ($t:expr, $s:expr) => {{
                assert_sig_to_str!($t, $s);
                assert_sig_from_str!($t, $s);
            }};
        }
        assert_sig_to_from_str!(Type::None, "_");
        assert_sig_to_from_str!(Type::Unknown, "X");
        assert_sig_to_from_str!(Type::Void, "v");
        assert_sig_to_from_str!(Type::Bool, "b");
        assert_sig_to_from_str!(Type::Int8, "c");
        assert_sig_to_from_str!(Type::UInt8, "C");
        assert_sig_to_from_str!(Type::Int16, "w");
        assert_sig_to_from_str!(Type::UInt16, "W");
        assert_sig_to_from_str!(Type::Int32, "i");
        assert_sig_to_from_str!(Type::UInt32, "I");
        assert_sig_to_from_str!(Type::Int64, "l");
        assert_sig_to_from_str!(Type::UInt64, "L");
        assert_sig_to_from_str!(Type::Float, "f");
        assert_sig_to_from_str!(Type::Double, "d");
        assert_sig_to_from_str!(Type::String, "s");
        assert_sig_to_from_str!(Type::Raw, "r");
        assert_sig_to_from_str!(Type::Object, "o");
        assert_sig_to_from_str!(Type::Dynamic, "m");
        assert_sig_to_from_str!(Type::Option(Type::Void.into()), "+v");
        assert_sig_to_from_str!(Type::List(Type::Int32.into()), "[i]");
        assert_sig_to_from_str!(Type::List(Type::Tuple(vec![]).into()), "[()]");
        assert_sig_to_from_str!(
            Type::Map {
                key: Type::Float.into(),
                value: Type::String.into(),
            },
            "{fs}"
        );
        assert_sig_to_from_str!(
            Type::Tuple(vec![Type::Float, Type::String, Type::UInt32]),
            "(fsI)"
        );
        assert_sig_to_from_str!(
            Type::Struct {
                name: String::new(),
                fields: indexmap![
                    "x".into() => Type::Float,
                    "y".into() => Type::Float
                ]
            },
            "(ff)<,x,y>"
        );
        // Annotations can be ignored.
        assert_sig_from_str!(Type::Tuple(vec![]), "()<>");
        assert_sig_from_str!(Type::Tuple(vec![Type::Int32]), "(i)<>");
        assert_sig_from_str!(Type::Tuple(vec![Type::Int32]), "(i)<,,,,,,,>");
        assert_sig_to_from_str!(
            Type::TupleStruct {
                name: "ExplorationMap".into(),
                elements: vec![
                    Type::List(Type::Tuple(vec![Type::Double, Type::Double]).into()),
                    Type::UInt64,
                ],
            },
            "([(dd)]L)<ExplorationMap>"
        );
        assert_sig_to_from_str!(
            Type::Struct {
                name: "ExplorationMap".into(),
                fields: indexmap![
                    "points".into() => Type::List(Type::Struct {
                        name: "Point".into(),
                        fields: indexmap![
                            "x".into() => Type::Double,
                            "y".into() => Type::Double
                        ],
                    }.into()),
                    "timestamp".into() => Type::UInt64,
                ],
            },
            "([(dd)<Point,x,y>]L)<ExplorationMap,points,timestamp>"
        );
        assert_sig_to_from_str!(Type::VarArgs(Type::Dynamic.into()), "#m");
        assert_sig_to_from_str!(Type::KwArgs(Type::Object.into()), "~o");
        // Some complex type for fun.
        assert_sig_to_from_str!(
            Type::Tuple(vec![
                Type::List(
                    Type::Map {
                        key: Type::Option(Type::Object.into()).into(),
                        value: Type::Raw.into(),
                    }
                    .into()
                ),
                Type::KwArgs(Type::Double.into()),
                Type::VarArgs(Type::Option(Type::Dynamic.into()).into()),
            ]),
            "([{+or}]~d#+m)"
        );
    }

    #[test]
    fn test_signature_from_str_errors() {
        assert_eq!("".parse::<Signature>(), Err(FromStrError::EndOfInput));
        assert_eq!(
            "u".parse::<Signature>(),
            Err(FromStrError::UnexpectedChar('u', "u".into()))
        );
        // Option
        assert_eq!(
            "+".parse::<Signature>(),
            Err(FromStrError::MissingOptionValueType("+".into()))
        );
        assert_eq!(
            "+[".parse::<Signature>(),
            Err(FromStrError::OptionValueTypeParsing(Box::new(
                FromStrError::MissingListValueType("[".into())
            )))
        );
        // VarArgs
        assert_eq!(
            "#".parse::<Signature>(),
            Err(FromStrError::MissingVarArgsValueType("#".into()))
        );
        assert_eq!(
            "#[".parse::<Signature>(),
            Err(FromStrError::VarArgsValueTypeParsing(Box::new(
                FromStrError::MissingListValueType("[".into())
            )))
        );
        // KwArgs
        assert_eq!(
            "~".parse::<Signature>(),
            Err(FromStrError::MissingKwArgsValueType("~".into()))
        );
        assert_eq!(
            "~[".parse::<Signature>(),
            Err(FromStrError::KwArgsValueTypeParsing(Box::new(
                FromStrError::MissingListValueType("[".into())
            )))
        );
        // Lists
        assert_eq!(
            "[".parse::<Signature>(),
            Err(FromStrError::MissingListValueType("[".into()))
        );
        assert_eq!(
            "[]".parse::<Signature>(),
            Err(FromStrError::MissingListValueType("[]".into()))
        );
        assert_eq!(
            "[i".parse::<Signature>(),
            Err(FromStrError::MissingListEnd("[i".into()))
        );
        assert_eq!(
            "[{i}]".parse::<Signature>(),
            Err(FromStrError::ListValueTypeParsing(Box::new(
                FromStrError::MissingMapValueType("{i}]".into())
            )))
        );
        // The error is `UnexpectedChar` and not `MissingTupleEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "[(]".parse::<Signature>(),
            Err(FromStrError::ListValueTypeParsing(Box::new(
                FromStrError::TupleElementTypeParsing(Box::new(FromStrError::UnexpectedChar(
                    ']',
                    "]".into()
                )))
            )))
        );
        // Maps
        assert_eq!(
            "{".parse::<Signature>(),
            Err(FromStrError::MissingMapKeyType("{".into()))
        );
        assert_eq!(
            "{}".parse::<Signature>(),
            Err(FromStrError::MissingMapKeyType("{}".into()))
        );
        assert_eq!(
            "{i}".parse::<Signature>(),
            Err(FromStrError::MissingMapValueType("{i}".into()))
        );
        assert_eq!(
            "{ii".parse::<Signature>(),
            Err(FromStrError::MissingMapEnd("{ii".into()))
        );
        assert_eq!(
            "{[]i}".parse::<Signature>(),
            Err(FromStrError::MapKeyTypeParsing(Box::new(
                FromStrError::MissingListValueType("[]i}".into())
            )))
        );
        assert_eq!(
            "{i[]}".parse::<Signature>(),
            Err(FromStrError::MapValueTypeParsing(Box::new(
                FromStrError::MissingListValueType("[]}".into())
            )))
        );
        // The error is `UnexpectedChar` and not `MissingListEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "{i[}".parse::<Signature>(),
            Err(FromStrError::MapValueTypeParsing(Box::new(
                FromStrError::ListValueTypeParsing(Box::new(FromStrError::UnexpectedChar(
                    '}',
                    "}".into()
                )))
            )))
        );
        // Tuples
        assert_eq!(
            "(".parse::<Signature>(),
            Err(FromStrError::MissingTupleEnd("(".into()))
        );
        assert_eq!(
            "(iii".parse::<Signature>(),
            Err(FromStrError::MissingTupleEnd("(iii".into()))
        );
        assert_eq!(
            "(i[i)".parse::<Signature>(),
            Err(FromStrError::TupleElementTypeParsing(Box::new(
                FromStrError::MissingListEnd("[i)".into())
            )))
        );
        // Tuples annotations
        assert_eq!(
            "(i)<".parse::<Signature>(),
            Err(FromStrError::MissingTupleAnnotationEnd(
                "<".into(),
                "(i)<".into()
            ))
        );
        assert_eq!(
            "(i)<S,a,b>".parse::<Signature>(),
            Err(FromStrError::Annotation(
                "expected 1 annotations but got 2".into(),
                "(i)<S,a,b>".into()
            ))
        );
        // The error is `UnexpectedChar` and not `MissingMapEnd` because we don't detect subtype
        // parsing.
        assert_eq!(
            "(i{i)".parse::<Signature>(),
            Err(FromStrError::TupleElementTypeParsing(Box::new(
                FromStrError::MapValueTypeParsing(Box::new(FromStrError::UnexpectedChar(
                    ')',
                    ")".into()
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
        let t = sig.into_type();
        assert_eq!(
            t,
            Type::Struct {
                name: "MetaObject".into(),
                fields: indexmap![
                    "methods".into() => Type::Map {
                        key: Type::UInt32.into(),
                        value: Type::Struct {
                            name: "MetaMethod".into(),
                            fields: indexmap![
                                "uid".into() => Type::UInt32,
                                "returnSignature".into() => Type::String,
                                "name".into() => Type::String,
                                "parametersSignature".into() => Type::String,
                                "description".into() => Type::String,
                                "parameters".into() => Type::List(
                                    Type::Struct {
                                        name: "MetaMethodParameter".into(),
                                        fields: indexmap![
                                            "name".into() => Type::String,
                                            "description".into() => Type::String
                                        ]
                                    }.into(),
                                ),
                                "returnDescription".into() => Type::String,
                            ]
                        }.into(),
                    },
                    "signals".into() => Type::Map {
                        key: Type::UInt32.into(),
                        value: Type::Struct {
                            name: "MetaSignal".into(),
                            fields: indexmap![
                                "uid".into() => Type::UInt32,
                                "name".into() => Type::String,
                                "signature".into() => Type::String,
                            ]
                        }.into()
                    }.into(),
                    "properties".into() => Type::Map {
                        key: Type::UInt32.into(),
                        value: Type::Struct {
                            name: "MetaProperty".into(),
                            fields: indexmap![
                                "uid".into() => Type::UInt32,
                                "name".into() => Type::String,
                                "signature".into() => Type::String,
                            ]
                        }.into()
                    }.into(),
                    "description".into() => Type::String,
                ]
            }
        );
    }

    #[test]
    fn test_signature_ser_de() {
        use serde_test::{assert_tokens, Token};
        assert_tokens(
            &Signature(Type::Struct {
                name: "Point".into(),
                fields: indexmap![
                    "x".into() => Type::Double,
                    "y".into() => Type::Double,
                ],
            }),
            &[Token::Str("(dd)<Point,x,y>")],
        )
    }
}
