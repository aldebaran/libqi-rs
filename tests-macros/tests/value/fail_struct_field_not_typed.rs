struct F;

#[derive(
    qi_macros::AsValue,
    qi_macros::FromValue,
    qi_macros::IntoValue,
    qi_macros::Reflect,
    qi_macros::StdTryFromValue,
)]
struct S {
    pub f: F,
}

fn main() {}
