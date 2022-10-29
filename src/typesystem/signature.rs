use super::r#type::{tuple, Tuple, Type};

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

    fn parse_type(iter: &mut std::str::Chars) -> Result<Type, FromStrError> {
        let input = iter.as_str();
        let c = iter.next().ok_or(FromStrError::EndOfInput)?;
        match c {
            Self::CHAR_NONE => Ok(Type::None),
            Self::CHAR_UNKNOWN => Ok(Type::Unknown),
            Self::CHAR_VOID => Ok(Type::Void),
            Self::CHAR_BOOL => Ok(Type::Bool),
            Self::CHAR_INT8 => Ok(Type::Int8),
            Self::CHAR_UINT8 => Ok(Type::UInt8),
            Self::CHAR_INT16 => Ok(Type::Int16),
            Self::CHAR_UINT16 => Ok(Type::UInt16),
            Self::CHAR_INT32 => Ok(Type::Int32),
            Self::CHAR_UINT32 => Ok(Type::UInt32),
            Self::CHAR_INT64 => Ok(Type::Int64),
            Self::CHAR_UINT64 => Ok(Type::UInt64),
            Self::CHAR_FLOAT => Ok(Type::Float),
            Self::CHAR_DOUBLE => Ok(Type::Double),
            Self::CHAR_STRING => Ok(Type::String),
            Self::CHAR_RAW => Ok(Type::Raw),
            Self::CHAR_OBJECT => Ok(Type::Object),
            Self::CHAR_DYNAMIC => Ok(Type::Dynamic),
            Self::CHAR_MARK_OPTION => Ok({
                let t = Self::parse_option_tail(iter, input)?;
                Type::option(t)
            }),
            Self::CHAR_LIST_BEGIN => Ok({
                let t = Self::parse_list_tail(iter, input)?;
                Type::list(t)
            }),
            Self::CHAR_MAP_BEGIN => Ok({
                let (key, value) = Self::parse_map_tail(iter, input)?;
                Type::map(key, value)
            }),
            Self::CHAR_TUPLE_BEGIN => {
                let tuple = Self::parse_tuple_tail(iter, input)?;
                fn parse_annotations<'c>(
                    mut iter: std::str::Chars<'c>,
                    tuple_start: &str,
                ) -> Result<
                    Option<((Option<String>, Option<Vec<String>>), std::str::Chars<'c>)>,
                    FromStrError,
                > {
                    match iter.next() {
                        Some(Signature::CHAR_ANNOTATIONS_BEGIN) => {
                            let (name, fields) =
                                Signature::parse_tuple_annotations_tail(&mut iter, tuple_start)?;
                            Ok(Some(((name, fields), iter)))
                        }
                        _ => Ok(None),
                    }
                }
                let tuple = match parse_annotations(iter.clone(), input)? {
                    Some(((name, fields), after_annot_iter)) => {
                        // Advance iter to after annotations.
                        *iter = after_annot_iter;
                        let elements = match fields {
                            Some(fields) => tuple
                                .elements
                                .name(fields)
                                .map_err(|err| FromStrError::Annotation(err, input.into()))?,
                            None => tuple.elements,
                        };
                        match name {
                            Some(name) => Tuple::named(name, elements),
                            None => Tuple::new(elements),
                        }
                    }
                    None => tuple,
                };
                Ok(Type::from(tuple))
            }
            Self::CHAR_MARK_VARSARGS => Ok({
                let t = Self::parse_varargs_tail(iter, input)?;
                Type::var_args(t)
            }),
            Self::CHAR_MARK_KWARGS => Ok({
                let t = Self::parse_kwargs_tail(iter, input)?;
                Type::kw_args(t)
            }),
            _ => Err(FromStrError::UnexpectedChar(c, input.into())),
        }
    }

    fn parse_option_tail(iter: &mut std::str::Chars, start: &str) -> Result<Type, FromStrError> {
        match Self::parse_type(iter) {
            Ok(t) => Ok(t),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingOptionValueType(start.into()),
                _ => FromStrError::OptionValueTypeParsing(Box::new(err)),
            }),
        }
        .into()
    }

    fn parse_varargs_tail(iter: &mut std::str::Chars, start: &str) -> Result<Type, FromStrError> {
        match Self::parse_type(iter) {
            Ok(t) => Ok(t),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingVarArgsValueType(start.into()),
                _ => FromStrError::VarArgsValueTypeParsing(Box::new(err)),
            }),
        }
    }

    fn parse_kwargs_tail(iter: &mut std::str::Chars, start: &str) -> Result<Type, FromStrError> {
        match Self::parse_type(iter) {
            Ok(t) => Ok(t),
            Err(err) => Err(match err {
                FromStrError::EndOfInput => FromStrError::MissingKwArgsValueType(start.into()),
                _ => FromStrError::KwArgsValueTypeParsing(Box::new(err)),
            }),
        }
    }

    fn parse_list_tail(iter: &mut std::str::Chars, start: &str) -> Result<Type, FromStrError> {
        let t = Self::parse_type(iter).map_err(|err| match err {
            FromStrError::UnexpectedChar(Self::CHAR_LIST_END, _) | FromStrError::EndOfInput => {
                FromStrError::MissingListValueType(start.into())
            }
            _ => FromStrError::ListValueTypeParsing(Box::new(err)),
        })?;
        match iter.next() {
            Some(Self::CHAR_LIST_END) => Ok(t),
            _ => Err(FromStrError::MissingListEnd(start.into())),
        }
    }

    fn parse_map_tail(
        iter: &mut std::str::Chars,
        start: &str,
    ) -> Result<(Type, Type), FromStrError> {
        let key = Self::parse_type(iter).map_err(|err| match err {
            FromStrError::UnexpectedChar(Self::CHAR_MAP_END, _) | FromStrError::EndOfInput => {
                FromStrError::MissingMapKeyType(start.into())
            }
            _ => FromStrError::MapKeyTypeParsing(Box::new(err)),
        })?;
        let value = Self::parse_type(iter).map_err(|err| match err {
            FromStrError::UnexpectedChar(Self::CHAR_MAP_END, _) => {
                FromStrError::MissingMapValueType(start.into())
            }
            _ => FromStrError::MapValueTypeParsing(Box::new(err)),
        })?;
        match iter.next() {
            Some(Self::CHAR_MAP_END) => Ok((key, value)),
            _ => Err(FromStrError::MissingMapEnd(start.into())),
        }
    }

    fn parse_tuple_tail(iter: &mut std::str::Chars, start: &str) -> Result<Tuple, FromStrError> {
        let mut fields = Vec::new();
        loop {
            match Self::parse_type(iter) {
                Ok(t) => fields.push(t),
                Err(err) => {
                    break match err {
                        FromStrError::UnexpectedChar(Self::CHAR_TUPLE_END, _) => {
                            Ok(Tuple::new(tuple::Elements::from_iter(fields)))
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
        tuple_start: &str,
    ) -> Result<(Option<String>, Option<Vec<String>>), FromStrError> {
        type Value = (Option<String>, Option<Vec<String>>);
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
            fn next(&mut self, value: &mut Value) {
                match std::mem::replace(self, Self::Field(None)) {
                    Self::Name(n) => {
                        value.0 = n;
                    }
                    Self::Field(f) => {
                        if let Some(f) = f {
                            let fields = &mut value.1;
                            let fields = fields.get_or_insert_with(|| Vec::new());
                            fields.push(f);
                        }
                    }
                }
            }
        }
        let mut value = Value::default();
        let mut state = State::Name(None);
        loop {
            match iter.next() {
                Some(Self::CHAR_ANNOTATIONS_SEP) => state.next(&mut value),
                Some(Self::CHAR_ANNOTATIONS_END) => {
                    state.next(&mut value);
                    break Ok(value);
                }
                Some(c) if c.is_alphanumeric() => state.push_char(c),
                Some(c) => break Err(FromStrError::UnexpectedChar(c, tuple_start.into())),
                None => break Err(FromStrError::MissingTupleAnnotationEnd(tuple_start.into())),
            }
        }
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
                // Skip annotations if both name and fields are absent.
                if t.name.is_none() && !t.has_fields() {
                    return Ok(());
                }
                write!(
                    f,
                    "{beg}{name}{fields}{end}",
                    beg = Self::CHAR_ANNOTATIONS_BEGIN,
                    end = Self::CHAR_ANNOTATIONS_END,
                    name = t.name.as_ref().unwrap_or(&String::new()),
                    fields = t
                        .fields()
                        .into_iter()
                        .flatten()
                        .map(|f| format!(",{name}", name = f.name))
                        .collect::<String>(),
                )
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
        Self::parse_type(&mut src.chars()).map(Self)
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

    #[error("annotation of structure failed: \"{0}\"")]
    Annotation(tuple::NameElementsError, String),

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
    use super::{super::r#type::tuple, *};

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
        assert_sig_to_from_str!(Type::option(Type::Void), "+v");
        assert_sig_to_from_str!(Type::list(Type::Int32), "[i]");
        assert_sig_to_from_str!(Type::list(Type::unit_tuple()), "[()]");
        assert_sig_to_from_str!(Type::map(Type::Float, Type::String), "{fs}");
        assert_sig_to_from_str!(
            Type::tuple_from_iter([Type::Float, Type::String, Type::UInt32]),
            "(fsI)"
        );
        assert_sig_to_from_str!(
            Type::tuple_from_iter([
                tuple::Field::new("x", Type::Float),
                tuple::Field::new("y", Type::Float)
            ]),
            "(ff)<,x,y>"
        );
        assert_sig_from_str!(Type::unit_tuple(), "()<>");
        assert_sig_from_str!(Type::tuple_from_iter([Type::Int32]), "(i)<>");
        assert_sig_from_str!(Type::tuple_from_iter([Type::Int32]), "(i)<,,,,,,,>");
        assert_sig_to_from_str!(
            Type::named_tuple_from_iter(
                "ExplorationMap",
                [
                    Type::list(Type::tuple_from_iter([Type::Double, Type::Double])),
                    Type::UInt64,
                ],
            ),
            "([(dd)]L)<ExplorationMap>"
        );
        assert_sig_to_from_str!(
            Type::named_tuple_from_iter(
                "ExplorationMap",
                [
                    tuple::Field::new(
                        "points",
                        Type::list(Type::named_tuple_from_iter(
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
            Type::tuple_from_iter([
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
            "(i)<S,a,b>".parse::<Signature>(),
            Err(FromStrError::Annotation(
                tuple::NameElementsError::BadNamesSize(1, 2),
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
            Type::named_tuple_from_iter(
                "MetaObject",
                [
                    tuple::Field::new(
                        "methods",
                        Type::map(
                            Type::UInt32,
                            Type::named_tuple_from_iter(
                                "MetaMethod",
                                [
                                    tuple::Field::new("uid", Type::UInt32),
                                    tuple::Field::new("returnSignature", Type::String),
                                    tuple::Field::new("name", Type::String),
                                    tuple::Field::new("parametersSignature", Type::String),
                                    tuple::Field::new("description", Type::String),
                                    tuple::Field::new(
                                        "parameters",
                                        Type::list(Type::named_tuple_from_iter(
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
                            Type::UInt32,
                            Type::named_tuple_from_iter(
                                "MetaSignal",
                                [
                                    tuple::Field::new("uid", Type::UInt32),
                                    tuple::Field::new("name", Type::String),
                                    tuple::Field::new("signature", Type::String),
                                ]
                            )
                        )
                    ),
                    tuple::Field::new(
                        "properties",
                        Type::map(
                            Type::UInt32,
                            Type::named_tuple_from_iter(
                                "MetaProperty",
                                [
                                    tuple::Field::new("uid", Type::UInt32),
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
        use serde_test::{assert_tokens, Token};
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
