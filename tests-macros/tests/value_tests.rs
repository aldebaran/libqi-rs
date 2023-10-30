use assert_matches::assert_matches;
use qi_value::{
    ty::{StructField, Tuple},
    Type,
};

#[derive(
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::IntoValue,
    qi_macros::Reflect,
    qi_macros::StdTryFromValue,
)]
#[allow(dead_code)]
struct Basic {
    s: String,
    b: bool,
    u: (),
    li: Vec<i32>,
}

#[test]
fn test_derive_reflect_basic() {
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

#[derive(
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::IntoValue,
    qi_macros::Reflect,
    qi_macros::StdTryFromValue,
)]
#[qi(transparent)]
#[allow(dead_code)]
struct Transparent {
    s: String,
}

#[test]
fn test_derive_reflect_transparent() {
    use qi_value::Reflect;
    assert_eq!(Transparent::ty(), Some(Type::String));
}

#[derive(qi_macros::Reflect)]
#[qi(rename_all = "camelCase")]
struct RenameAll {
    #[allow(dead_code)]
    my_field_has_a_name_with_underscores: i32,
}

#[test]
fn test_derive_reflect_rename_all() {
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
    t.compile_fail("tests/value/fail_struct_field_not_typed.rs");
    t.compile_fail("tests/value/fail_enum.rs");
    t.compile_fail("tests/value/fail_struct_transparent_more_than_one_field.rs");
}
