#[derive(qi_macros::Reflect, qi_macros::ToValue, qi_macros::IntoValue, qi_macros::FromValue)]
enum E {
    A,
    B(i32),
    C { c: String },
}

fn main() {}
