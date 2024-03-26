use assert_matches::assert_matches;
use qi_messaging::{capabilities, CapabilitiesMap};
use qi_value::{Dynamic, IntoValue, Value};

#[test]
fn capability_map_merge_with() {
    let mut m = CapabilitiesMap::from_iter([
        ("A".to_owned(), Dynamic(true.into_value())),
        ("B".to_owned(), Dynamic(true.into_value())),
        ("C".to_owned(), Dynamic(false.into_value())),
        ("D".to_owned(), Dynamic(false.into_value())),
        ("E".to_owned(), Dynamic(true.into_value())),
        ("F".to_owned(), Dynamic(false.into_value())),
    ]);
    let m2 = CapabilitiesMap::from_iter([
        ("A".to_owned(), Dynamic(true.into_value())),
        ("B".to_owned(), Dynamic(false.into_value())),
        ("C".to_owned(), Dynamic(true.into_value())),
        ("D".to_owned(), Dynamic(false.into_value())),
        ("G".to_owned(), Dynamic(true.into_value())),
        ("H".to_owned(), Dynamic(false.into_value())),
    ]);
    capabilities::intersect(&mut m, &m2);
    assert_matches!(m.get("A"), Some(Dynamic(Value::Bool(true))));
    assert_matches!(m.get("B"), Some(Dynamic(Value::Bool(false))));
    assert_matches!(m.get("C"), Some(Dynamic(Value::Bool(false))));
    assert_matches!(m.get("D"), Some(Dynamic(Value::Bool(false))));
    assert_matches!(m.get("E"), None);
    assert_matches!(m.get("F"), None);
    assert_matches!(m.get("G"), None);
    assert_matches!(m.get("H"), None);
    assert_matches!(m.get("I"), None);
}
