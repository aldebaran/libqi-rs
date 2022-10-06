use super::Value;

#[derive(Debug, PartialEq, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tuple {
    pub name: Option<String>,
    pub fields: Fields,
}

impl Tuple {
    pub fn has_named_fields(&self) -> bool {
        matches!(self.fields, Fields::Named(..))
    }

    pub fn named_fields(&self) -> Option<&Vec<NamedField>> {
        match &self.fields {
            Fields::Named(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn unnamed_fields(&self) -> Option<&Vec<Value>> {
        match &self.fields {
            Fields::Unnamed(fields) => Some(fields),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum Fields {
    Unnamed(Vec<Value>),
    Named(Vec<NamedField>),
}

impl IntoIterator for Fields {
    type Item = Value;
    type IntoIter = FieldsIntoIter<
        std::vec::IntoIter<Value>,
        std::vec::IntoIter<NamedField>,
        Value,
        NamedField,
    >;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Fields::Unnamed(values) => FieldsIntoIter::unnamed(values.into_iter()),
            Fields::Named(named_field) => {
                FieldsIntoIter::named(named_field.into_iter(), |field| field.value)
            }
        }
    }
}

impl<'a> IntoIterator for &'a Fields {
    type Item = &'a Value;
    type IntoIter = FieldsIntoIter<
        std::slice::Iter<'a, Value>,
        std::slice::Iter<'a, NamedField>,
        &'a Value,
        &'a NamedField,
    >;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Fields::Unnamed(values) => FieldsIntoIter::unnamed(values.into_iter()),
            Fields::Named(named_field) => {
                FieldsIntoIter::named(named_field.into_iter(), |field| &field.value)
            }
        }
    }
}

impl<'a> IntoIterator for &'a mut Fields {
    type Item = &'a mut Value;
    type IntoIter = FieldsIntoIter<
        std::slice::IterMut<'a, Value>,
        std::slice::IterMut<'a, NamedField>,
        &'a mut Value,
        &'a mut NamedField,
    >;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Fields::Unnamed(values) => FieldsIntoIter::unnamed(values.into_iter()),
            Fields::Named(named_field) => {
                FieldsIntoIter::named(named_field.into_iter(), |field| &mut field.value)
            }
        }
    }
}

enum FieldsIntoIterIter<VIter, NFIter, V, N> {
    Unnamed(VIter),
    Named(NFIter, fn(N) -> V),
}

pub struct FieldsIntoIter<VIter, NFIter, V, N> {
    iter: FieldsIntoIterIter<VIter, NFIter, V, N>,
    phantom: std::marker::PhantomData<(V, N)>,
}

impl<VIter, NFIter, V, N> FieldsIntoIter<VIter, NFIter, V, N>
where
    VIter: Iterator<Item = V>,
    NFIter: Iterator<Item = N>,
{
    fn unnamed(iter: VIter) -> Self {
        Self {
            iter: FieldsIntoIterIter::Unnamed(iter),
            phantom: std::marker::PhantomData,
        }
    }

    fn named(iter: NFIter, field_to_value: fn(N) -> V) -> Self {
        Self {
            iter: FieldsIntoIterIter::Named(iter, field_to_value),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<VIter, NFIter, V, N> Iterator for FieldsIntoIter<VIter, NFIter, V, N>
where
    VIter: Iterator<Item = V>,
    NFIter: Iterator<Item = N>,
{
    type Item = V;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.iter {
            FieldsIntoIterIter::Unnamed(iter) => iter.next(),
            FieldsIntoIterIter::Named(iter, field_to_value) => iter.next().map(field_to_value),
        }
    }
}

impl FromIterator<Value> for Fields {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self::Unnamed(iter.into_iter().collect())
    }
}

impl From<Vec<Value>> for Fields {
    fn from(v: Vec<Value>) -> Self {
        Self::Unnamed(v)
    }
}

impl FromIterator<NamedField> for Fields {
    fn from_iter<T: IntoIterator<Item = NamedField>>(iter: T) -> Self {
        Self::Named(iter.into_iter().collect())
    }
}

impl From<Vec<NamedField>> for Fields {
    fn from(v: Vec<NamedField>) -> Self {
        Self::Named(v)
    }
}

impl Default for Fields {
    fn default() -> Self {
        Self::Unnamed(Default::default())
    }
}

#[derive(Debug, PartialEq, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct NamedField {
    pub name: String,
    pub value: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn sample_tuple_unnamed_fields() -> (Tuple, Vec<Value>) {
        let fields = vec![Value::Bool(true), Value::Raw(vec![48, 49, 50])];
        let t = Tuple {
            name: None,
            fields: Fields::Unnamed(fields.clone()),
        };
        (t, fields)
    }

    fn sample_tuple_named_fields() -> (Tuple, Vec<NamedField>) {
        let fields = vec![
            NamedField {
                name: "cookies".to_string(),
                value: Value::Int32(42),
            },
            NamedField {
                name: "muffins".to_string(),
                value: Value::String("croissants".to_string()),
            },
        ];
        let t = Tuple {
            name: None,
            fields: Fields::Named(fields.clone()),
        };
        (t, fields)
    }

    #[test]
    fn test_tuple_has_named_fields_unnamed() {
        let (t, _) = sample_tuple_unnamed_fields();
        assert!(!t.has_named_fields());
    }

    #[test]
    fn test_tuple_has_named_fields_named() {
        let (t, _) = sample_tuple_named_fields();
        assert!(t.has_named_fields());
    }

    #[test]
    fn test_tuple_named_fields_named() {
        let (t, expected) = sample_tuple_named_fields();
        assert_eq!(t.named_fields(), Some(&expected));
    }

    #[test]
    fn test_tuple_named_fields_unnamed() {
        let (t, _) = sample_tuple_unnamed_fields();
        assert_eq!(t.named_fields(), None);
    }

    #[test]
    fn test_tuple_unnamed_fields_named() {
        let (t, _) = sample_tuple_named_fields();
        assert_eq!(t.unnamed_fields(), None);
    }

    #[test]
    fn test_tuple_unnamed_fields_unnamed() {
        let (t, expected) = sample_tuple_unnamed_fields();
        assert_eq!(t.unnamed_fields(), Some(&expected));
    }

    #[test]
    fn test_fields_into_iterator_named() {
        let (t, _) = sample_tuple_named_fields();
        let fields = t.fields.into_iter().collect::<Vec<_>>();
        assert_eq!(
            fields,
            vec![Value::Int32(42), Value::String("croissants".to_string())]
        );
    }

    #[test]
    fn test_fields_into_iterator_unnamed() {
        let (t, _) = sample_tuple_unnamed_fields();
        let fields = t.fields.into_iter().collect::<Vec<_>>();
        assert_eq!(
            fields,
            vec![Value::Bool(true), Value::Raw(vec![48, 49, 50])]
        );
    }
}
