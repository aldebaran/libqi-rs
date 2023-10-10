#[derive(qi_derive::Typed)]
enum E {
    A,
    B(i32),
    C { c: String },
}

fn main() {}
