/// The type of a value in the `qi` type system.
///
/// The absence of a type equals to the unit `Dynamic` type, which is the set of all types.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Type {
    Unit,
    Bool,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float32,
    Float64,
    String,
    Raw,
    Object,
    Option(Option<Box<Type>>),
    List(Option<Box<Type>>),
    VarArgs(Option<Box<Type>>),
    Map {
        key: Option<Box<Type>>,
        value: Option<Box<Type>>,
    },
    Tuple(Tuple),
}

/// Defaults constructs a type as a unit type.
impl Default for Type {
    fn default() -> Self {
        Self::Unit
    }
}

impl From<Tuple> for Type {
    fn from(tuple: Tuple) -> Self {
        Type::Tuple(tuple)
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Unit => f.write_str("unit"),
            Type::Bool => f.write_str("bool"),
            Type::Int8 => f.write_str("int8"),
            Type::UInt8 => f.write_str("uint8"),
            Type::Int16 => f.write_str("int16"),
            Type::UInt16 => f.write_str("uint16"),
            Type::Int32 => f.write_str("int32"),
            Type::UInt32 => f.write_str("uint32"),
            Type::Int64 => f.write_str("int64"),
            Type::UInt64 => f.write_str("uint64"),
            Type::Float32 => f.write_str("float32"),
            Type::Float64 => f.write_str("float64"),
            Type::String => f.write_str("string"),
            Type::Raw => f.write_str("raw"),
            Type::Object => f.write_str("object"),
            Type::Option(t) => {
                f.write_str("option(")?;
                DisplayTypeOption(t).fmt(f)?;
                f.write_str(")")
            }
            Type::List(t) => {
                f.write_str("list(")?;
                DisplayTypeOption(t).fmt(f)?;
                f.write_str(")")
            }
            Type::VarArgs(t) => {
                f.write_str("varargs(")?;
                DisplayTypeOption(t).fmt(f)?;
                f.write_str(")")
            }
            Type::Map { key, value } => {
                f.write_str("map(")?;
                DisplayTypeOption(key).fmt(f)?;
                f.write_str(",")?;
                DisplayTypeOption(value).fmt(f)?;
                f.write_str(")")
            }
            Type::Tuple(t) => t.fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct DisplayTypeOption<'a, T>(pub &'a Option<T>);

impl<T> std::fmt::Display for DisplayTypeOption<'_, T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(ty) => ty.fmt(f),
            None => f.write_str("dynamic"),
        }
    }
}

#[derive(Debug)]
pub struct DisplayTypeTuple<'a, T>(pub &'a Vec<Option<T>>);

impl<T> std::fmt::Display for DisplayTypeTuple<'_, T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        for t in self.0 {
            DisplayTypeOption(t).fmt(f)?;
        }
        f.write_str(")")
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Tuple {
    Tuple(Vec<Option<Type>>),
    TupleStruct {
        name: String,
        elements: Vec<Option<Type>>,
    },
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
}

impl Tuple {
    pub fn new() -> Self {
        Self::Tuple(vec![])
    }

    pub fn struct_from_annotations_of_elements(
        annotations: StructAnnotations,
        elements: Vec<Option<Type>>,
    ) -> Result<Self, ZipStructFieldsSizeError> {
        let tuple = if let Some(field_names) = annotations.field_names {
            Tuple::Struct {
                name: annotations.name,
                fields: zip_struct_fields(field_names, elements)?,
            }
        } else {
            Tuple::TupleStruct {
                name: annotations.name,
                elements,
            }
        };
        Ok(tuple)
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Tuple(t) => t.len(),
            Self::TupleStruct { elements, .. } => elements.len(),
            Self::Struct { fields, .. } => fields.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Tuple(t) => t.is_empty(),
            Self::TupleStruct { elements, .. } => elements.is_empty(),
            Self::Struct { fields, .. } => fields.is_empty(),
        }
    }

    pub fn element_types(&self) -> Vec<Option<Type>> {
        match self {
            Self::Tuple(t) => t.clone(),
            Self::TupleStruct { elements, .. } => elements.clone(),
            Self::Struct { fields, .. } => fields.iter().map(|field| field.ty.clone()).collect(),
        }
    }

    pub fn name(&self) -> Option<String> {
        match self {
            Self::Tuple(_) => None,
            Self::TupleStruct { name, .. } | Self::Struct { name, .. } => Some(name.clone()),
        }
    }

    pub fn field_names(&self) -> Option<Vec<String>> {
        match self {
            Self::Tuple(_) | Self::TupleStruct { .. } => None,
            Self::Struct { fields, .. } => {
                Some(fields.iter().map(|field| field.name.clone()).collect())
            }
        }
    }

    pub fn annotations(&self) -> Option<StructAnnotations> {
        match self {
            Self::Tuple(_) => None,
            Self::TupleStruct { name, .. } => Some(StructAnnotations {
                name: name.clone(),
                field_names: None,
            }),
            Self::Struct { name, fields } => Some(StructAnnotations {
                name: name.clone(),
                field_names: Some(fields.iter().map(|field| field.name.clone()).collect()),
            }),
        }
    }
}

impl Default for Tuple {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Tuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("tuple(")?;
        for (idx, element) in self.element_types().into_iter().enumerate() {
            if idx > 0 {
                f.write_str(",")?;
            }
            DisplayTypeOption(&element).fmt(f)?;
        }
        f.write_str(")")?;
        if let Some(annotations) = self.annotations() {
            annotations.fmt(f)?;
        }
        Ok(())
    }
}

impl<I, T> From<I> for Tuple
where
    I: IntoIterator<Item = T>,
    T: Into<Option<Type>>,
{
    fn from(iter: I) -> Self {
        Self::Tuple(iter.into_iter().map(Into::into).collect())
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct StructField {
    pub name: String,
    pub ty: Option<Type>,
}

impl<S, T> From<(S, T)> for StructField
where
    S: Into<String>,
    T: Into<Option<Type>>,
{
    fn from(v: (S, T)) -> Self {
        Self {
            name: v.0.into(),
            ty: v.1.into(),
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct StructAnnotations {
    pub name: String,
    pub field_names: Option<Vec<String>>,
}

impl std::fmt::Display for StructAnnotations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{name}", name = self.name)?;
        if let Some(fields) = &self.field_names {
            for (idx, field) in fields.iter().enumerate() {
                if idx > 0 {
                    f.write_str(",")?;
                }
                f.write_str(field)?;
            }
        }
        f.write_str(">")?;
        Ok(())
    }
}

pub fn option<T>(t: T) -> Type
where
    T: Into<Option<Type>>,
{
    Type::Option(t.into().map(Box::new))
}

pub fn varargs<T>(t: T) -> Type
where
    T: Into<Option<Type>>,
{
    Type::VarArgs(t.into().map(Box::new))
}

pub fn list<T>(t: T) -> Type
where
    T: Into<Option<Type>>,
{
    Type::List(t.into().map(Box::new))
}

pub fn map<K, V>(key: K, value: V) -> Type
where
    K: Into<Option<Type>>,
    V: Into<Option<Type>>,
{
    Type::Map {
        key: key.into().map(Box::new),
        value: value.into().map(Box::new),
    }
}

pub fn tuple<I, F>(fields: I) -> Type
where
    I: IntoIterator<Item = F>,
    F: Into<Option<Type>>,
{
    Type::Tuple(Tuple::Tuple(fields.into_iter().map(Into::into).collect()))
}

pub fn unit_tuple() -> Type {
    Type::Tuple(Tuple::Tuple(vec![]))
}

pub fn struct_ty<N, I, F>(name: N, fields: I) -> Type
where
    N: Into<String>,
    I: IntoIterator<Item = F>,
    F: Into<StructField>,
{
    Type::Tuple(Tuple::Struct {
        name: name.into(),
        fields: fields.into_iter().map(Into::into).collect(),
    })
}

pub fn tuple_struct<N, I, F>(name: N, elements: I) -> Type
where
    N: Into<String>,
    I: IntoIterator<Item = F>,
    F: Into<Option<Type>>,
{
    Type::Tuple(Tuple::TupleStruct {
        name: name.into(),
        elements: elements.into_iter().map(Into::into).collect(),
    })
}

pub(crate) fn zip_struct_fields<N, E>(
    names: N,
    elements: E,
) -> Result<Vec<StructField>, ZipStructFieldsSizeError>
where
    N: IntoIterator,
    N::Item: Into<String>,
    E: IntoIterator,
    E::Item: Into<Option<Type>>,
{
    let mut names = names.into_iter().fuse();
    let mut elements = elements.into_iter().fuse();
    let mut fields = Vec::new();
    loop {
        match (names.next(), elements.next()) {
            (Some(name), Some(element)) => fields.push(StructField {
                name: name.into(),
                ty: element.into(),
            }),
            (None, None) => break Ok(fields),
            (name, element) => {
                break Err(ZipStructFieldsSizeError {
                    name_count: fields.len() + if name.is_some() { 1 } else { 0 } + names.count(),
                    element_count: fields.len()
                        + if element.is_some() { 1 } else { 0 }
                        + elements.count(),
                })
            }
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, thiserror::Error)]
#[error("error zipping structure fields names and elements, got {name_count} names for {element_count} elements")]
pub struct ZipStructFieldsSizeError {
    pub name_count: usize,
    pub element_count: usize,
}

pub fn common_type(t1: Option<Type>, t2: Option<Type>) -> Option<Type> {
    t1.zip(t2).filter(|(t1, t2)| t1 == t2).map(|(t1, _)| t1)
}

pub fn reduce_type<C>(c: C) -> Option<Type>
where
    C: IntoIterator<Item = Type>,
{
    c.into_iter().map(Some).reduce(common_type).flatten()
}

pub fn reduce_map_types<C>(c: C) -> (Option<Type>, Option<Type>)
where
    C: IntoIterator<Item = (Type, Type)>,
{
    c.into_iter()
        .map(|(k, v)| (Some(k), Some(v)))
        .reduce(|(ck, cv), (k, v)| (common_type(ck, k), common_type(cv, v)))
        .unwrap_or((None, None))
}
