struct F;

#[derive(qi_macros::Reflect, qi_macros::ToValue, qi_macros::IntoValue, qi_macros::FromValue)]
struct S {
    pub f: F,
}

fn main() {}
