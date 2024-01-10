#[derive(qi_macros::Reflect, qi_macros::ToValue, qi_macros::IntoValue, qi_macros::FromValue)]
#[qi(value = "qi_value", transparent)]
enum E {
    A,
    B(i32),
    C { c: String },
}

fn main() {}
