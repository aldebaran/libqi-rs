use std::borrow::Cow;

use assert_matches::assert_matches;
use qi_value::{
    ty::{StructField, Tuple},
    AsRaw, IntoValue, Type, Value,
};

#[derive(
    Debug,
    PartialEq,
    qi_macros::Reflect,
    qi_macros::ToValue,
    qi_macros::IntoValue,
    qi_macros::FromValue,
)]
#[qi(value = "qi_value")]
#[allow(dead_code)]
struct Basic {
    s: String,
    b: bool,
    u: (),
    li: Vec<i32>,
}

#[test]
fn test_basic_derive_reflect() {
    use qi_value::Reflect;
    assert_matches!(
        Basic::ty(),
        Some(Type::Tuple(Tuple::Struct { name, fields })) => {
            assert_eq!(name, "Basic");
            assert_matches!(fields.as_slice(), [s, b, u, li] => {
                assert_matches!(
                    s,
                    StructField {
                        name,
                        ty: Some(Type::String)
                    } => assert_eq!(name, "s")
                );
                assert_matches!(
                    b,
                    StructField {
                        name,
                        ty: Some(Type::Bool)
                    } => assert_eq!(name, "b")
                );
                assert_matches!(
                    u,
                    StructField {
                        name,
                        ty: Some(Type::Unit)
                    } => assert_eq!(name, "u")
                );
                assert_matches!(
                    li,
                    StructField {
                        name,
                        ty: Some(Type::List(value_type))
                    } => {
                        assert_eq!(name, "li");
                        assert_eq!(value_type.as_deref(), Some(&Type::Int32));
                    }
                );
            });
        }
    );
}

#[test]
fn test_basic_derive_to_value() {
    use qi_value::ToValue;
    let b = Basic {
        s: "cookies".to_owned(),
        b: true,
        u: (),
        li: vec![4, 5, 6],
    };
    assert_matches!(
        b.to_value(),
        Value::Tuple(fields) => {
            assert_matches!(
                fields.as_slice(),
                [
                    Value::String(Cow::Borrowed(s)),
                    Value::Bool(true),
                    Value::Unit,
                    Value::List(li)
                ] => {
                    assert_eq!(*s, "cookies");
                    assert_eq!(
                        li.as_slice(),
                        &[
                            Value::Int32(4),
                            Value::Int32(5),
                            Value::Int32(6)
                        ]
                    );
                }
            );
        }
    );
}

#[test]
fn test_basic_derive_into_value() {
    use qi_value::IntoValue;
    let b = Basic {
        s: "muffins".to_owned(),
        b: false,
        u: (),
        li: vec![1, 1, 2, 3, 5, 8],
    };
    assert_matches!(
        b.into_value(),
        Value::Tuple(fields) => {
            assert_matches!(
                fields.as_slice(),
                [
                    Value::String(Cow::Owned(s)),
                    Value::Bool(false), Value::Unit, Value::List(li)
                ] => {
                    assert_eq!(s, "muffins");
                    assert_eq!(
                        li.as_slice(),
                        &[
                            Value::Int32(1),
                            Value::Int32(1),
                            Value::Int32(2),
                            Value::Int32(3),
                            Value::Int32(5),
                            Value::Int32(8)
                        ]);
                }
            );
        }
    );
}

#[test]
fn test_basic_derive_from_value() {
    use qi_value::{FromValue, IntoValue};
    let value = ("cheesecake", true, (), [10, 9, 8, 7].as_slice()).into_value();
    assert_eq!(
        Basic::from_value(value).unwrap(),
        Basic {
            s: "cheesecake".to_owned(),
            b: true,
            u: (),
            li: vec![10, 9, 8, 7],
        }
    );
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    qi_macros::Reflect,
    qi_macros::ToValue,
    qi_macros::IntoValue,
    qi_macros::FromValue,
)]
#[qi(value = "qi_value")]
#[allow(dead_code)]
struct Borrows<'a, 'b> {
    s: &'a str,
    #[qi(as_raw)]
    r: &'b [u8],
}

#[test]
fn test_borrows_derive_reflect() {
    use qi_value::Reflect;
    assert_matches!(
        Borrows::ty(),
        Some(Type::Tuple(Tuple::Struct { name, fields })) => {
            assert_eq!(name, "Borrows");
            assert_matches!(
                fields.as_slice(),
                [
                    StructField { name: s, ty: Some(Type::String) },
                    StructField { name: r, ty: Some(Type::Raw) }
                ] => {
                    assert_eq!(s, "s");
                    assert_eq!(r, "r");
                }
            )
        }
    );
}

#[test]
fn test_borrows_derive_to_value() {
    use qi_value::ToValue;
    let sbuf = String::from("cupcakes");
    let rbuf = vec![1, 20, 100, 200];
    let b = Borrows { s: &sbuf, r: &rbuf };
    assert_matches!(
        b.to_value(),
        Value::Tuple(tuple) => {
            assert_matches!(
                tuple.as_slice(),
                [
                    Value::String(Cow::Borrowed(s)),
                    Value::Raw(Cow::Borrowed([1, 20, 100, 200]))
                ] => {
                    assert_eq!(*s, "cupcakes");
                }
            )
        }
    );
}

#[test]
fn test_borrows_derive_into_value() {
    use qi_value::IntoValue;
    let sbuf = String::from("apples");
    let rbuf = vec![7, 5, 3, 2, 1];
    let b = Borrows { s: &sbuf, r: &rbuf };
    assert_matches!(
        b.into_value(),
        Value::Tuple(tuple) => {
            assert_matches!(
                tuple.as_slice(),
                [
                    Value::String(Cow::Borrowed(s)),
                    Value::Raw(Cow::Borrowed([7, 5, 3, 2, 1]))
                ] => {
                    assert_eq!(*s, "apples");
                }
            )
        }
    );
}

#[test]
fn test_borrows_derive_from_value() {
    use qi_value::FromValue;
    let sbuf = String::from("bananas");
    let rbuf = vec![255, 128, 64, 32, 16];
    let value = (&sbuf, AsRaw(&rbuf)).into_value();
    assert_eq!(
        Borrows::from_value(value).unwrap(),
        Borrows { s: &sbuf, r: &rbuf }
    );
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    qi_macros::Reflect,
    qi_macros::ToValue,
    qi_macros::IntoValue,
    qi_macros::FromValue,
)]
#[qi(value = "qi_value", transparent)]
#[allow(dead_code)]
struct Transparent {
    s: String,
}

#[test]
fn test_transparent_derive_reflect() {
    use qi_value::Reflect;
    assert_eq!(Transparent::ty(), Some(Type::String));
}

#[test]
fn test_transparent_derive_to_value() {
    use qi_value::ToValue;
    assert_eq!(
        Transparent {
            s: "mangoes".to_owned()
        }
        .to_value(),
        Value::String("mangoes".into()),
    )
}

#[test]
fn test_transparent_derive_into_value() {
    use qi_value::IntoValue;
    assert_eq!(
        Transparent {
            s: "pears".to_owned()
        }
        .into_value(),
        Value::String("pears".into()),
    )
}

#[test]
fn test_transparent_derive_from_value() {
    use qi_value::FromValue;
    let value = "grapes".into_value();
    assert_eq!(
        Transparent::from_value(value).unwrap(),
        Transparent {
            s: "grapes".to_owned(),
        }
    );
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    qi_macros::Reflect,
    qi_macros::ToValue,
    qi_macros::IntoValue,
    qi_macros::FromValue,
)]
#[qi(value = "qi_value", transparent)]
struct Empty;

#[test]
fn test_empty_derive_reflect() {
    use qi_value::Reflect;
    assert_eq!(Transparent::ty(), Some(Type::String));
}

#[test]
fn test_empty_derive_to_value() {
    use qi_value::ToValue;
    assert_eq!(
        Transparent {
            s: "mangoes".to_owned()
        }
        .to_value(),
        Value::String("mangoes".into()),
    )
}

#[test]
fn test_empty_derive_into_value() {
    use qi_value::IntoValue;
    assert_eq!(
        Transparent {
            s: "pears".to_owned()
        }
        .into_value(),
        Value::String("pears".into()),
    )
}

#[test]
fn test_empty_derive_from_value() {
    use qi_value::FromValue;
    let value = "grapes".into_value();
    assert_eq!(
        Transparent::from_value(value).unwrap(),
        Transparent {
            s: "grapes".to_owned(),
        }
    );
}

#[derive(qi_macros::Reflect, qi_macros::ToValue, qi_macros::IntoValue, qi_macros::FromValue)]
#[qi(value = "qi_value", rename_all = "camelCase")]
struct RenameAll {
    my_field_has_a_name_with_underscores: i32,
}

#[test]
fn test_derive_rename_all_reflect() {
    use qi_value::Reflect;
    assert_matches!(
        RenameAll::ty(),
        Some(Type::Tuple(Tuple::Struct { name, fields })) => {
            assert_eq!(name, "RenameAll");
            assert_matches!(fields.as_slice(), [f] => {
                assert_matches!(
                    f,
                    StructField {
                        name,
                        ty: Some(Type::Int32)
                    } => assert_eq!(name, "myFieldHasANameWithUnderscores")
                );
            });
        }
    );
}

#[test]
fn test_derive_typed_build() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/value/fail_struct_field_not_impl.rs");
    t.compile_fail("tests/value/fail_enum.rs");
    t.compile_fail("tests/value/fail_struct_transparent_more_than_one_field.rs");
}
