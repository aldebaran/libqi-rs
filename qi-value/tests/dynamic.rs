use qi_value::Dynamic;
use serde_test::{assert_tokens, Token};
use std::collections::BTreeMap;

#[test]
fn serde_struct() {
    #[derive(PartialEq, Debug, qi_macros::Reflect, qi_macros::ToValue, qi_macros::FromValue)]
    #[qi(value(crate = "qi_value"))]
    struct MyStruct {
        an_int: i32,
        #[qi(value(as_raw))]
        a_raw: Vec<u8>,
        an_option: Option<BTreeMap<String, Vec<bool>>>,
    }
    assert_tokens(
        &Dynamic(MyStruct {
            an_int: 42,
            a_raw: vec![1, 2, 3],
            an_option: Some(BTreeMap::from_iter([
                ("true_true".to_owned(), vec![true, true]),
                ("false_true".to_owned(), vec![false, true]),
                ("true_false".to_owned(), vec![true, false]),
                ("false_false".to_owned(), vec![false, false]),
            ])),
        }),
        &[
            Token::Struct {
                name: "Dynamic",
                len: 2,
            },
            Token::Str("signature"),
            Token::Str("(ir+{s[b]})<MyStruct,an_int,a_raw,an_option>"),
            Token::Str("value"),
            Token::Tuple { len: 3 },
            Token::I32(42),
            Token::BorrowedBytes(&[1, 2, 3]),
            Token::Some,
            Token::Map { len: Some(4) },
            Token::Str("false_false"),
            Token::Seq { len: Some(2) },
            Token::Bool(false),
            Token::Bool(false),
            Token::SeqEnd,
            Token::Str("false_true"),
            Token::Seq { len: Some(2) },
            Token::Bool(false),
            Token::Bool(true),
            Token::SeqEnd,
            Token::Str("true_false"),
            Token::Seq { len: Some(2) },
            Token::Bool(true),
            Token::Bool(false),
            Token::SeqEnd,
            Token::Str("true_true"),
            Token::Seq { len: Some(2) },
            Token::Bool(true),
            Token::Bool(true),
            Token::SeqEnd,
            Token::MapEnd,
            Token::TupleEnd,
            Token::StructEnd,
        ],
    );
}

#[test]
fn serde_with() {
    #[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    struct DynString(#[serde(with = "qi_value::dynamic")] String);
    assert_tokens(
        &DynString("Cookies are good".to_owned()),
        &[
            Token::Struct {
                name: "Dynamic",
                len: 2,
            },
            Token::Str("signature"),
            Token::Str("s"),
            Token::Str("value"),
            Token::BorrowedStr("Cookies are good"),
            Token::StructEnd,
        ],
    )
}
