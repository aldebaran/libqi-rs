use super::r#type::Type;

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
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
    const CHAR_MARK_VARSARGS: char = '#';
    const CHAR_MARK_KWARGS: char = '~';
    const CHAR_ANNOTATIONS_BEGIN: char = '<';
    const CHAR_ANNOTATIONS_SEP: char = ',';
    const CHAR_ANNOTATIONS_END: char = '>';

    fn parse(iter: &mut std::str::Chars) -> Result<Self, FromStrError> {
        let input = iter.as_str();
        let c = iter.next().ok_or(FromStrError::EndOfInput)?;
        match c {
            Self::CHAR_NONE => Ok(Self(Type::None)),
            Self::CHAR_UNKNOWN => Ok(Self(Type::Unknown)),
            Self::CHAR_VOID => Ok(Self(Type::Void)),
            Self::CHAR_BOOL => Ok(Self(Type::Bool)),
            Self::CHAR_INT8 => Ok(Self(Type::Int8)),
            Self::CHAR_UINT8 => Ok(Self(Type::UInt8)),
            Self::CHAR_INT16 => Ok(Self(Type::Int16)),
            Self::CHAR_UINT16 => Ok(Self(Type::UInt16)),
            Self::CHAR_INT32 => Ok(Self(Type::Int32)),
            Self::CHAR_UINT32 => Ok(Self(Type::UInt32)),
            Self::CHAR_INT64 => Ok(Self(Type::Int64)),
            Self::CHAR_UINT64 => Ok(Self(Type::UInt64)),
            Self::CHAR_FLOAT => Ok(Self(Type::Float)),
            Self::CHAR_DOUBLE => Ok(Self(Type::Double)),
            Self::CHAR_STRING => Ok(Self(Type::String)),
            Self::CHAR_RAW => Ok(Self(Type::Raw)),
            Self::CHAR_OBJECT => Ok(Self(Type::Object)),
            Self::CHAR_DYNAMIC => Ok(Self(Type::Dynamic)),
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
            Ok(sig) => Ok(Self(Type::option(sig.0))),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingOptionValueType(start.into()),
                _ => FromStrError::OptionValueTypeParsing(Box::new(err)),
            }),
        }
        .into()
    }

    fn parse_varargs_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        match Self::parse(iter) {
            Ok(Self(t)) => Ok(Self(Type::var_args(t))),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingVarArgsValueType(start.into()),
                _ => FromStrError::VarArgsValueTypeParsing(Box::new(err)),
            }),
        }
    }

    fn parse_kwargs_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        match Self::parse(iter) {
            Ok(Self(t)) => Ok(Self(Type::kw_args(t))),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingKwArgsValueType(start.into()),
                _ => FromStrError::KwArgsValueTypeParsing(Box::new(err)),
            }),
        }
    }

    fn parse_list_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        let sig = Self::parse(iter).map_err(|err| match err {
            FromStrError::UnexpectedChar(Self::CHAR_LIST_END, _) | FromStrError::EndOfInput => {
                FromStrError::MissingListValueType(start.into())
            }
            _ => FromStrError::ListValueTypeParsing(Box::new(err)),
        })?;
        let t = sig.into_type();
        match iter.next() {
            Some(Self::CHAR_LIST_END) => Ok(Self(Type::list(t))),
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
        let key = key.into_type();
        let value = Self::parse(iter).map_err(|err| match err {
            FromStrError::UnexpectedChar(Self::CHAR_MAP_END, _) => {
                FromStrError::MissingMapValueType(start.into())
            }
            _ => FromStrError::MapValueTypeParsing(Box::new(err)),
        })?;
        let value = value.into_type();
        match iter.next() {
            Some(Self::CHAR_MAP_END) => Ok(Self(Type::map(key, value))),
            _ => Err(FromStrError::MissingMapEnd(start.into())),
        }
    }

    fn parse_tuple_tail(iter: &mut std::str::Chars, start: &str) -> Result<Self, FromStrError> {
        let mut fields = Vec::new();
        loop {
            match Self::parse(iter) {
                Ok(Signature(t)) => fields.push(t),
                Err(err) => {
                    break match err {
                        FromStrError::UnexpectedChar(Self::CHAR_TUPLE_END, _) => {
                            Ok(Self(Type::tuple(fields)))
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

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        match &self.0 {
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
            Type::Option(o) => write!(
                f,
                "{mark}{o}",
                mark = Self::CHAR_MARK_OPTION,
                o = Self((**o).clone())
            ),
            Type::List(t) => write!(
                f,
                "{beg}{t}{end}",
                beg = Self::CHAR_LIST_BEGIN,
                t = Self((**t).clone()),
                end = Self::CHAR_LIST_END
            ),
            Type::Map { key, value } => write!(
                f,
                "{beg}{key}{value}{end}",
                beg = Self::CHAR_MAP_BEGIN,
                key = Self((**key).clone()),
                value = Self((**value).clone()),
                end = Self::CHAR_MAP_END
            ),
            Type::Tuple(t) => {
                write!(
                    f,
                    "{beg}{ts}{end}",
                    beg = Self::CHAR_TUPLE_BEGIN,
                    end = Self::CHAR_TUPLE_END,
                    ts = t
                        .into_iter()
                        .fold(String::new(), |s, t| s + &Self(t.clone()).to_string())
                )?;
                Ok(())
                //match t {
                //    Tuple{ name: None, elements: Elements::Raw(_) } => Ok(()),

                //}
            }
            Type::VarArgs(t) => write!(
                f,
                "{mark}{t}",
                mark = Self::CHAR_MARK_VARSARGS,
                t = Self(*t.clone())
            ),
            Type::KwArgs(t) => write!(
                f,
                "{mark}{t}",
                mark = Self::CHAR_MARK_KWARGS,
                t = Self(*t.clone())
            ),
        }
    }
}

impl std::str::FromStr for Signature {
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
        <&str>::deserialize(deserializer)?
            .parse()
            .map_err(|e| serde::de::Error::custom(e))
    }
}

#[cfg(test)]
mod tests {
    use super::{super::r#type::tuple, *};
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_signature_to_from_string() {
        macro_rules! assert_sig_to_from_str {
            ($t:expr, $s:expr) => {{
                assert_eq!(
                    Signature($t).to_string(),
                    $s,
                    "Left is {t:?}.to_string(), Right is {s:?}",
                    t = $t,
                    s = $s
                );
                assert_eq!(
                    $s.parse::<Signature>().map(|s| s.into_type()),
                    Ok($t),
                    "Left is {s:?}.parse(), Right is {t:?}",
                    s = $s,
                    t = $t
                );
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
        assert_sig_to_from_str!(Type::option(Type::Void), "+v");
        assert_sig_to_from_str!(Type::list(Type::Int32), "[i]");
        assert_sig_to_from_str!(Type::map(Type::Float, Type::String), "{fs}");
        assert_sig_to_from_str!(
            Type::tuple([Type::Float, Type::String, Type::UInt32]),
            "(fsI)"
        );
        assert_sig_to_from_str!(
            Type::tuple([
                tuple::Field::new("x", Type::Float),
                tuple::Field::new("y", Type::Float)
            ]),
            "(fsI)<,x,y>"
        );
        assert_sig_to_from_str!(
            Type::named_tuple(
                "ExplorationMap",
                [
                    Type::list(Type::tuple([Type::Double, Type::Double])),
                    Type::UInt64,
                ],
            ),
            "([(dd)]L)<ExplorationMap>"
        );
        assert_sig_to_from_str!(
            Type::named_tuple(
                "ExplorationMap",
                [
                    tuple::Field::new(
                        "points",
                        Type::list(Type::named_tuple(
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
        assert_sig_to_from_str!(Type::var_args(Type::Dynamic), "#m");
        assert_sig_to_from_str!(Type::kw_args(Type::Object), "~o");
        // Some complex type for fun.
        assert_sig_to_from_str!(
            Type::tuple([
                Type::list(Type::map(Type::option(Type::Object), Type::Raw)),
                Type::kw_args(Type::Double),
                Type::var_args(Type::option(Type::Dynamic)),
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
            Err(FromStrError::MissingTupleAnnotationEnd("(i)<".into()))
        );
        assert_eq!(
            "(i)<>".parse::<Signature>(),
            Err(FromStrError::MissingTupleAnnotationStructName(
                "(i)<>".into()
            ))
        );
        assert_eq!(
            "(i)<S>".parse::<Signature>(),
            Err(FromStrError::MissingTupleAnnotationFieldName(
                "(i)<S>".into()
            ))
        );
        assert_eq!(
            "(i)<S,a,b>".parse::<Signature>(),
            Err(FromStrError::UnexpectedTupleAnnotationFieldName(
                1,
                "b>".into(),
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
        let input = "({I(Issss[(ss)<MetaMethodParameter,name,description>]s)\
                     <MetaMethod,uid,returnSignature,name,parametersSignature,\
                     description,parameters,returnDescription>}{I(Iss)<MetaSignal,\
                     uid,name,signature>}{I(Iss)<MetaProperty,uid,name,signature>}s)\
                     <MetaObject,methods,signals,properties,description>";
        let sig: Signature = input.parse().unwrap();
        let t = sig.into_type();
        assert_eq!(
            t,
            Type::named_tuple(
                "MetaObject",
                [
                    tuple::Field::new(
                        "methods",
                        Type::map(
                            Type::Int64,
                            Type::named_tuple(
                                "MetaMethod",
                                [
                                    tuple::Field::new("uid", Type::Int64),
                                    tuple::Field::new("returnSignature", Type::String),
                                    tuple::Field::new("name", Type::String),
                                    tuple::Field::new("parametersSignature", Type::String),
                                    tuple::Field::new("description", Type::String),
                                    tuple::Field::new(
                                        "parameters",
                                        Type::list(Type::named_tuple(
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
                            Type::named_tuple(
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
                            Type::named_tuple(
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
    fn test_signature_ser_de() {
        assert_tokens(
            &Signature(Type::Tuple(tuple::Tuple::named(
                "Point",
                tuple::Elements::from_iter([
                    tuple::Field::new("x", Type::Double),
                    tuple::Field::new("y", Type::Double),
                ]),
            ))),
            &[Token::Str("(dd)<Point,x,y>")],
        )
    }
}
