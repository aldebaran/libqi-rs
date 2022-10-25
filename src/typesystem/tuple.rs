#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Tuple<T> {
    pub name: Option<String>,
    pub elements: Elements<T>,
}

impl<T> Tuple<T> {
    pub fn named<S, E>(name: S, elements: E) -> Self
    where
        S: Into<String>,
        E: Into<Elements<T>>,
    {
        Self {
            name: Some(name.into()),
            elements: elements.into(),
        }
    }

    pub fn anonymous<E>(elements: E) -> Self
    where
        E: Into<Elements<T>>,
    {
        Self {
            name: None,
            elements: elements.into(),
        }
    }

    pub fn has_fields(&self) -> bool {
        matches!(self.elements, Elements::Fields(..))
    }

    pub fn fields(&self) -> Option<&Vec<Field<T>>> {
        match &self.elements {
            Elements::Fields(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn raw_elements(&self) -> Option<&Vec<T>> {
        match &self.elements {
            Elements::Raw(elems) => Some(elems),
            _ => None,
        }
    }
}

impl<T> IntoIterator for Tuple<T> {
    type Item = <Elements<T> as IntoIterator>::Item;
    type IntoIter = <Elements<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Tuple<T> {
    type Item = <&'a Elements<T> as IntoIterator>::Item;
    type IntoIter = <&'a Elements<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.elements).into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Tuple<T> {
    type Item = <&'a mut Elements<T> as IntoIterator>::Item;
    type IntoIter = <&'a mut Elements<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.elements).into_iter()
    }
}

#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum Elements<T> {
    Raw(Vec<T>),
    Fields(Vec<Field<T>>),
}

impl<T> Default for Elements<T> {
    fn default() -> Self {
        Self::Raw(Default::default())
    }
}

impl<T> IntoIterator for Elements<T> {
    type Item = T;
    type IntoIter =
        ElementsIntoIter<std::vec::IntoIter<T>, std::vec::IntoIter<Field<T>>, T, Field<T>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Elements::Raw(elems) => ElementsIntoIter::raw_elements(elems.into_iter()),
            Elements::Fields(field) => {
                ElementsIntoIter::fields(field.into_iter(), |field| field.element)
            }
        }
    }
}

impl<'a, T> IntoIterator for &'a Elements<T> {
    type Item = &'a T;
    type IntoIter = ElementsIntoIter<
        std::slice::Iter<'a, T>,
        std::slice::Iter<'a, Field<T>>,
        &'a T,
        &'a Field<T>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Elements::Raw(elems) => ElementsIntoIter::raw_elements(elems.iter()),
            Elements::Fields(fields) => {
                ElementsIntoIter::fields(fields.iter(), |field| &field.element)
            }
        }
    }
}

impl<'a, T> IntoIterator for &'a mut Elements<T> {
    type Item = &'a mut T;
    type IntoIter = ElementsIntoIter<
        std::slice::IterMut<'a, T>,
        std::slice::IterMut<'a, Field<T>>,
        &'a mut T,
        &'a mut Field<T>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Elements::Raw(elems) => ElementsIntoIter::raw_elements(elems.iter_mut()),
            Elements::Fields(fields) => {
                ElementsIntoIter::fields(fields.iter_mut(), |field| &mut field.element)
            }
        }
    }
}

impl<T> FromIterator<T> for Elements<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::Raw(iter.into_iter().collect())
    }
}

impl<T> FromIterator<Field<T>> for Elements<T> {
    fn from_iter<I: IntoIterator<Item = Field<T>>>(iter: I) -> Self {
        Self::Fields(iter.into_iter().collect())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum ElementsIntoIterIter<VIter, NFIter, V, N> {
    Raw(VIter),
    Fields(NFIter, fn(N) -> V),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ElementsIntoIter<VIter, NFIter, V, N> {
    iter: ElementsIntoIterIter<VIter, NFIter, V, N>,
    phantom: std::marker::PhantomData<(V, N)>,
}

impl<ElemIter, FieldIter, E, F> ElementsIntoIter<ElemIter, FieldIter, E, F>
where
    ElemIter: Iterator<Item = E>,
    FieldIter: Iterator<Item = F>,
{
    fn raw_elements(iter: ElemIter) -> Self {
        Self {
            iter: ElementsIntoIterIter::Raw(iter),
            phantom: std::marker::PhantomData,
        }
    }

    fn fields(iter: FieldIter, field_to_elem: fn(F) -> E) -> Self {
        Self {
            iter: ElementsIntoIterIter::Fields(iter, field_to_elem),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<ElemIter, FieldIter, E, F> Iterator for ElementsIntoIter<ElemIter, FieldIter, E, F>
where
    ElemIter: Iterator<Item = E>,
    FieldIter: Iterator<Item = F>,
{
    type Item = E;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.iter {
            ElementsIntoIterIter::Raw(iter) => iter.next(),
            ElementsIntoIterIter::Fields(iter, field_to_elem) => iter.next().map(field_to_elem),
        }
    }
}

#[derive(
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct Field<T> {
    pub name: String,
    pub element: T,
}

impl<T> Field<T> {
    pub fn new<S, E>(name: S, element: E) -> Self
    where
        S: Into<String>,
        E: Into<T>,
    {
        Self {
            name: name.into(),
            element: element.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn sample_tuple_raw_elements() -> (Tuple<i32>, Vec<i32>) {
        let elems = vec![42, 239];
        let t = Tuple {
            name: None,
            elements: Elements::Raw(elems.clone()),
        };
        (t, elems)
    }

    fn sample_tuple_fields() -> (Tuple<i32>, Vec<Field<i32>>) {
        let fields = vec![
            Field {
                name: "cookies".to_string(),
                element: 32910,
            },
            Field {
                name: "muffins".to_string(),
                element: -21393,
            },
        ];
        let t = Tuple {
            name: None,
            elements: Elements::Fields(fields.clone()),
        };
        (t, fields)
    }

    #[test]
    fn test_tuple_has_fields_raw_elements() {
        let (t, _) = sample_tuple_raw_elements();
        assert!(!t.has_fields());
    }

    #[test]
    fn test_tuple_has_fields_fields() {
        let (t, _) = sample_tuple_fields();
        assert!(t.has_fields());
    }

    #[test]
    fn test_tuple_fields_fields() {
        let (t, expected) = sample_tuple_fields();
        assert_eq!(t.fields(), Some(&expected));
    }

    #[test]
    fn test_tuple_fields_raw_elements() {
        let (t, _) = sample_tuple_raw_elements();
        assert_eq!(t.fields(), None);
    }

    #[test]
    fn test_tuple_raw_elements_fields() {
        let (t, _) = sample_tuple_fields();
        assert_eq!(t.raw_elements(), None);
    }

    #[test]
    fn test_tuple_raw_elements_raw_elements() {
        let (t, expected) = sample_tuple_raw_elements();
        assert_eq!(t.raw_elements(), Some(&expected));
    }

    #[test]
    fn test_tuple_into_iterator_fields() {
        let (t, _) = sample_tuple_fields();
        let elems = t.into_iter().collect::<Vec<_>>();
        assert_eq!(elems, vec![32910, -21393]);
    }

    #[test]
    fn test_tuple_into_iterator_raw_elements() {
        let (t, _) = sample_tuple_raw_elements();
        let elems = t.into_iter().collect::<Vec<_>>();
        assert_eq!(elems, vec![42, 239]);
    }

    #[test]
    fn test_fields_into_iterator_fields() {
        let (t, _) = sample_tuple_fields();
        let elems = t.elements.into_iter().collect::<Vec<_>>();
        assert_eq!(elems, vec![32910, -21393]);
    }

    #[test]
    fn test_fields_into_iterator_raw_elements() {
        let (t, _) = sample_tuple_raw_elements();
        let elems = t.elements.into_iter().collect::<Vec<_>>();
        assert_eq!(elems, vec![42, 239]);
    }
}
