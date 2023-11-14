use qi_value::{Dynamic, Value};
use std::{cmp::Ordering, collections::HashMap};

pub type CapabilitiesMap<'a> = HashMap<String, Dynamic<Value<'a>>>;

pub fn intersect_with<'a, 'l>(
    lhs: &'l mut CapabilitiesMap<'a>,
    rhs: &CapabilitiesMap<'_>,
) -> &'l mut CapabilitiesMap<'a> {
    for (key, other_value) in rhs.iter() {
        if let Some(value) = lhs.get_mut(key) {
            // Prefer values from this map when no ordering can be made. Only use the other map
            // values if they are strictly inferior.
            if let Some(Ordering::Less) = other_value.partial_cmp(value) {
                *value = other_value.clone().into_owned();
            }
        }
    }

    // Only keep capabilities that were present in `other`.
    lhs.retain(|k, _| rhs.get(k).is_some());

    lhs
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use qi_value::IntoValue;

    #[test]
    fn test_capability_map_merge_with() {
        let mut m = CapabilitiesMap::from_iter([
            ("A".to_owned(), true.into_dynamic_value()),
            ("B".to_owned(), true.into_dynamic_value()),
            ("C".to_owned(), false.into_dynamic_value()),
            ("D".to_owned(), false.into_dynamic_value()),
            ("E".to_owned(), true.into_dynamic_value()),
            ("F".to_owned(), false.into_dynamic_value()),
        ]);
        let m2 = CapabilitiesMap::from_iter([
            ("A".to_owned(), true.into_dynamic_value()),
            ("B".to_owned(), false.into_dynamic_value()),
            ("C".to_owned(), true.into_dynamic_value()),
            ("D".to_owned(), false.into_dynamic_value()),
            ("G".to_owned(), true.into_dynamic_value()),
            ("H".to_owned(), false.into_dynamic_value()),
        ]);
        intersect_with(&mut m, &m2);
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
}
