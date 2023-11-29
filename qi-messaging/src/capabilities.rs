use qi_value::{Dynamic, Value};
use std::{cmp::Ordering, collections::HashMap};

pub type CapabilitiesMap = HashMap<String, Dynamic<Value<'static>>>;

pub trait CapabilitiesMapExt: private::Sealed {
    fn intersected_with(self, other: &CapabilitiesMap) -> Self;
}

impl CapabilitiesMapExt for CapabilitiesMap {
    fn intersected_with(mut self, other: &CapabilitiesMap) -> Self {
        for (key, other_value) in other.iter() {
            if let Some(value) = self.get_mut(key) {
                // Prefer values from this map when no ordering can be made. Only use the other map
                // values if they are strictly inferior.
                if let Some(Ordering::Less) = other_value.partial_cmp(value) {
                    *value = other_value.clone().into_owned();
                }
            }
        }

        // Only keep capabilities that were present in `other`.
        self.retain(|k, _| other.get(k).is_some());

        self
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::CapabilitiesMap {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use qi_value::IntoValue;

    #[test]
    fn test_capability_map_merge_with() {
        let m = CapabilitiesMap::from_iter([
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
        let m = m.intersected_with(&m2);
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
