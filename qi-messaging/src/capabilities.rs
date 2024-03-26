use qi_value::{Dynamic, Value};
use std::{cmp::Ordering, collections::HashMap};

pub type CapabilitiesMap = HashMap<String, Dynamic<Value<'static>>>;

pub fn intersect(this: &mut CapabilitiesMap, other: &CapabilitiesMap) {
    for (key, other_value) in other.iter() {
        if let Some(value) = this.get_mut(key) {
            // Prefer values from this map when no ordering can be made. Only use the other map
            // values if they are strictly inferior.
            if let Some(Ordering::Less) = other_value.partial_cmp(value) {
                *value = other_value.clone().into_owned();
            }
        }
    }

    // Only keep capabilities that were present in `other`.
    this.retain(|k, _| other.get(k).is_some());
}
