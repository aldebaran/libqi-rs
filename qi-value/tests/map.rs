use qi_value::Map;
use serde_test::{assert_tokens, Token};

#[test]
fn from_iter_removes_duplicates() {
    assert_eq!(
        Map::from_iter([(42, "forty-two"), (13, "thirteen"), (42, "quarante-deux")]),
        Map::from_iter([(42, "quarante-deux"), (13, "thirteen")]),
    );
}

#[test]
fn serde() {
    assert_tokens(
        &Map::from_iter([(32i16, "trente deux"), (34i16, "trente quatre")]),
        &[
            Token::Map { len: Some(2) },
            Token::I16(32),
            Token::BorrowedStr("trente deux"),
            Token::I16(34),
            Token::BorrowedStr("trente quatre"),
            Token::MapEnd,
        ],
    );
}
