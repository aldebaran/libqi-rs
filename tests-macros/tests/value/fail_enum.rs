#[derive(
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::IntoValue,
    qi_macros::Reflect,
    qi_macros::StdTryFromValue,
)]
enum E {
    A,
    B(i32),
    C { c: String },
}

fn main() {}
