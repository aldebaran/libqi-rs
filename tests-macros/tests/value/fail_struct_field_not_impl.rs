struct F;

#[derive(qi_macros::Reflect, qi_macros::ToValue, qi_macros::IntoValue, qi_macros::FromValue)]
#[qi(value = "qi_value", transparent)]
struct S {
    pub f: F,
}

fn main() {}
