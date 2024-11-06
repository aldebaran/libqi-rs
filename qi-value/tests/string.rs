use qi_value::value::String;
use serde_test::{assert_tokens, Token};

#[test]
fn serde() {
    let str = "cookies";
    assert_tokens(&String::from(str), &[Token::Str(str)]);
}

#[test]
fn serde_borrowed() {
    let str = "muffins";
    assert_tokens(&String::from(str), &[Token::BorrowedStr(str)]);
}

#[test]
fn serde_owned() {
    let str = "cupcakes";
    assert_tokens(&String::from(str), &[Token::String(str)]);
}
