#[derive(qi_macros::Reflect, qi_macros::ToValue, qi_macros::IntoValue, qi_macros::FromValue)]
#[qi(value(crate = "qi_value", transparent))]
struct S {
    a: i32,
    b: i32,
}

fn main() {}
