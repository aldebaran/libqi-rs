use qi_value::{
    signature::{AnnotationsError, ParseError},
    ty, Signature, Type,
};

#[test]
fn to_from_string() {
    macro_rules! assert_sig_to_str {
        ($t:expr, $s:expr) => {{
            assert_eq!(
                Signature::new($t.into()).to_string(),
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
    assert_sig_from_to_str!(Type::option_of(Type::Unit), "+v");
    assert_sig_from_to_str!(Type::varargs_of(None), "#m");
    assert_sig_from_to_str!(Type::list_of(Type::Int32), "[i]");
    assert_sig_from_to_str!(Type::list_of(Type::unit_tuple()), "[()]");
    assert_sig_from_to_str!(Type::map_of(Type::Float32, Type::String), "{fs}");
    assert_sig_from_to_str!(
        Type::tuple_of([Type::Float32, Type::String, Type::UInt32]),
        "(fsI)"
    );
    assert_sig_from_to_str!(
        Type::tuple_struct_of(
            "ExplorationMap",
            [
                Type::list_of(Type::tuple_of([Type::Float64, Type::Float64])),
                Type::UInt64
            ]
        ),
        "([(dd)]L)<ExplorationMap>"
    );
    assert_sig_from_to_str!(
        Type::struct_of(
            "ExplorationMap",
            [
                (
                    "points",
                    Type::list_of(Type::struct_of(
                        "Point",
                        [("x", Type::Float64), ("y", Type::Float64),]
                    ))
                ),
                ("timestamp", Type::UInt64)
            ]
        ),
        "([(dd)<Point,x,y>]L)<ExplorationMap,points,timestamp>"
    );
    // Underscores in structure and field names are allowed.
    // Spaces between structure or field names are trimmed.
    assert_sig_from_to_str!(
        "(i)<   A_B ,  c_d   >" =>
        Type::struct_of("A_B", [("c_d", Type::Int32)]) =>
        "(i)<A_B,c_d>"
    );
    // Annotations can be ignored if the struct name is missing.
    assert_sig_from_to_str!("()<>" => Type::unit_tuple() => "()");
    assert_sig_from_to_str!("(i)<>" => Type::tuple_of([Type::Int32]) => "(i)");
    assert_sig_from_to_str!("(i)<,,,,,,,>" => Type::tuple_of([Type::Int32]) => "(i)");
    assert_sig_from_to_str!("(ff)<,x,y>" => Type::tuple_of([Type::Float32, Type::Float32]) => "(ff)");
    // Some complex type for fun.
    assert_sig_from_to_str!(
        Type::tuple_of([
            Type::list_of(Type::map_of(Type::option_of(Type::Object), Type::Raw)),
            Type::varargs_of(Type::option_of(None))
        ]),
        "([{+or}]#+m)"
    );
}

#[test]
fn from_str_errors() {
    assert_eq!("".parse::<Signature>(), Err(ParseError::EndOfInput));
    assert_eq!(
        "u".parse::<Signature>(),
        Err(ParseError::UnexpectedChar('u', "u".to_owned()))
    );
    // Option
    assert_eq!(
        "+".parse::<Signature>(),
        Err(ParseError::MissingOptionValueType("+".to_owned()))
    );
    assert_eq!(
        "+[".parse::<Signature>(),
        Err(ParseError::OptionValueTypeParsing(Box::new(
            ParseError::MissingListValueType("[".to_owned())
        )))
    );
    // VarArgs
    assert_eq!(
        "#".parse::<Signature>(),
        Err(ParseError::MissingVarArgsValueType("#".to_owned()))
    );
    assert_eq!(
        "#[".parse::<Signature>(),
        Err(ParseError::VarArgsValueTypeParsing(Box::new(
            ParseError::MissingListValueType("[".to_owned())
        )))
    );
    // Lists
    assert_eq!(
        "[".parse::<Signature>(),
        Err(ParseError::MissingListValueType("[".to_owned()))
    );
    assert_eq!(
        "[]".parse::<Signature>(),
        Err(ParseError::MissingListValueType("[]".to_owned()))
    );
    assert_eq!(
        "[i".parse::<Signature>(),
        Err(ParseError::MissingListEnd("[i".to_owned()))
    );
    assert_eq!(
        "[{i}]".parse::<Signature>(),
        Err(ParseError::ListValueTypeParsing(Box::new(
            ParseError::MissingMapValueType("{i}]".to_owned())
        )))
    );
    // The error is `UnexpectedChar` and not `MissingTupleEnd` because we don't detect subtype
    // parsing.
    assert_eq!(
        "[(]".parse::<Signature>(),
        Err(ParseError::ListValueTypeParsing(Box::new(
            ParseError::TupleElementTypeParsing(Box::new(ParseError::UnexpectedChar(
                ']',
                "]".to_owned()
            )))
        )))
    );
    // Maps
    assert_eq!(
        "{".parse::<Signature>(),
        Err(ParseError::MissingMapKeyType("{".to_owned()))
    );
    assert_eq!(
        "{}".parse::<Signature>(),
        Err(ParseError::MissingMapKeyType("{}".to_owned()))
    );
    assert_eq!(
        "{i}".parse::<Signature>(),
        Err(ParseError::MissingMapValueType("{i}".to_owned()))
    );
    assert_eq!(
        "{ii".parse::<Signature>(),
        Err(ParseError::MissingMapEnd("{ii".to_owned()))
    );
    assert_eq!(
        "{[]i}".parse::<Signature>(),
        Err(ParseError::MapKeyTypeParsing(Box::new(
            ParseError::MissingListValueType("[]i}".to_owned())
        )))
    );
    assert_eq!(
        "{i[]}".parse::<Signature>(),
        Err(ParseError::MapValueTypeParsing(Box::new(
            ParseError::MissingListValueType("[]}".to_owned())
        )))
    );
    // The error is `UnexpectedChar` and not `MissingListEnd` because we don't detect subtype
    // parsing.
    assert_eq!(
        "{i[}".parse::<Signature>(),
        Err(ParseError::MapValueTypeParsing(Box::new(
            ParseError::ListValueTypeParsing(Box::new(ParseError::UnexpectedChar(
                '}',
                "}".to_owned()
            )))
        )))
    );
    // Tuples
    assert_eq!(
        "(".parse::<Signature>(),
        Err(ParseError::MissingTupleEnd("(".to_owned()))
    );
    assert_eq!(
        "(iii".parse::<Signature>(),
        Err(ParseError::MissingTupleEnd("(iii".to_owned()))
    );
    assert_eq!(
        "(i[i)".parse::<Signature>(),
        Err(ParseError::TupleElementTypeParsing(Box::new(
            ParseError::MissingListEnd("[i)".to_owned())
        )))
    );
    // Tuples annotations
    assert_eq!(
        "(i)<".parse::<Signature>(),
        Err(ParseError::Annotations {
            annotations: "<".to_owned(),
            tuple: "(i)<".to_owned(),
            source: AnnotationsError::MissingTupleAnnotationEnd
        })
    );
    assert_eq!(
        "(i)<S,a,b>".parse::<Signature>(),
        Err(ParseError::Annotations {
            annotations: "<S,a,b>".to_owned(),
            tuple: "(i)<S,a,b>".to_owned(),
            source: AnnotationsError::ZipError(ty::ZipStructFieldsSizeError {
                name_count: 2,
                element_count: 1
            }),
        })
    );
    //   - Only ASCII is supported
    assert_eq!(
        "(i)<越>".parse::<Signature>(),
        Err(ParseError::Annotations {
            annotations: "<越>".to_owned(),
            tuple: "(i)<越>".to_owned(),
            source: AnnotationsError::UnexpectedChar('越'),
        })
    );

    // The error is `UnexpectedChar` and not `MissingMapEnd` because we don't detect subtype
    // parsing.
    assert_eq!(
        "(i{i)".parse::<Signature>(),
        Err(ParseError::TupleElementTypeParsing(Box::new(
            ParseError::MapValueTypeParsing(Box::new(ParseError::UnexpectedChar(
                ')',
                ")".to_owned()
            )))
        )))
    );
}

#[test]
fn serde() {
    use serde_test::{assert_tokens, Token};
    assert_tokens(
        &Signature::new(Some(Type::struct_of(
            "Point",
            [("x", Type::Float64), ("y", Type::Float64)],
        ))),
        &[Token::Str("(dd)<Point,x,y>")],
    )
}
