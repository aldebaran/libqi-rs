#[derive(
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::IntoValue,
    qi_macros::Reflect,
    qi_macros::StdTryFromValue,
)]
#[qi(transparent)]
struct S {
    a: i32,
    b: i32,
}

fn main() {}
