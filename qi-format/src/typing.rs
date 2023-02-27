use crate::String;

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
    Dynamic,
    Option(Box<Type>),
    List(Box<Type>),
    VarArgs(Box<Type>),
    Map { key: Box<Type>, value: Box<Type> },
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
            Type::Dynamic => f.write_str("dynamic"),
            Type::Option(t) => write!(f, "option({t})"),
            Type::List(t) => write!(f, "list({t})"),
            Type::VarArgs(t) => write!(f, "varargs({t})"),
            Type::Map { key, value } => write!(f, "map({key},{value})"),
            Type::Tuple(t) => t.fmt(f),
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Tuple {
    elements: Vec<Type>,
    annotations: Option<TupleAnnotations>,
}

impl Tuple {
    pub fn new() -> Self {
        Self::unit()
    }

    pub fn unit() -> Self {
        Self {
            elements: vec![],
            annotations: None,
        }
    }

    pub fn from_element_types(elements: Vec<Type>) -> Self {
        Self {
            elements,
            annotations: None,
        }
    }

    pub fn from_element_types_with_annotations(
        elements: Vec<Type>,
        annotations: TupleAnnotations,
    ) -> Result<Self, TupleAnnotationsError> {
        if let Some(fields) = &annotations.fields {
            let field_count = fields.len();
            if field_count != elements.len() {
                return Err(TupleAnnotationsError::BadLength {
                    expected: elements.len(),
                    actual: field_count,
                });
            }
        }
        Ok(Self {
            elements,
            annotations: Some(annotations),
        })
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn element_types(&self) -> &Vec<Type> {
        &self.elements
    }

    pub fn annotations(&self) -> Option<&TupleAnnotations> {
        self.annotations.as_ref()
    }
}

impl IntoIterator for Tuple {
    type Item = Type;
    type IntoIter = std::vec::IntoIter<Type>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

impl<'t> IntoIterator for &'t Tuple {
    type Item = &'t Type;
    type IntoIter = std::slice::Iter<'t, Type>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

impl std::fmt::Display for Tuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("tuple(")?;
        for (idx, element) in self.elements.iter().enumerate() {
            if idx > 0 {
                f.write_str(",")?;
            }
            element.fmt(f)?;
        }
        f.write_str(")")?;
        if let Some(annotations) = &self.annotations {
            annotations.fmt(f)?;
        }
        Ok(())
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TupleAnnotations {
    pub name: String,
    pub fields: Option<Vec<String>>,
}

impl std::fmt::Display for TupleAnnotations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{name}", name = self.name)?;
        if let Some(fields) = &self.fields {
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

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum TupleAnnotationsError {
    #[error("expected {expected} annotations but got {actual}")]
    BadLength { expected: usize, actual: usize },
}

pub fn option(t: Type) -> Type {
    Type::Option(Box::new(t))
}

pub fn list(t: Type) -> Type {
    Type::List(Box::new(t))
}

#[macro_export]
macro_rules! map_type {
    ($key:expr => $value:expr) => {
        $crate::typing::Type::Map {
            key: Box::new($key),
            value: Box::new($value),
        }
    };
}

#[macro_export]
macro_rules! tuple_type {
    ($($t:expr),+ $(,)*) => {
        $crate::typing::Type::Tuple(
            $crate::typing::Tuple::from_element_types(
                vec![$($t),+]
            )
        )
    };
    () => {
        $crate::typing::Type::Tuple(
            $crate::typing::Tuple::new()
        )
    };
}

#[macro_export]
macro_rules! annotated_tuple_type {
    ($name:expr => { $($f:expr => $t:expr),+ $(,)* }) => {
        $crate::annotated_tuple_type!(inner
            $name,
            vec![$($t),+],
            Some(vec![$($crate::String::from($f)),+])
        )
    };
    ($name:expr => { $($t:expr),+ $(,)* }) => {
        $crate::annotated_tuple_type!(inner
            $name,
            vec![$($t),+],
            None
        )
    };
    (inner $name:expr, $types:expr, $fields:expr) => {
        $crate::typing::Type::Tuple(
            $crate::typing::Tuple::from_element_types_with_annotations(
                $types,
                $crate::typing::TupleAnnotations {
                    name: $crate::String::from($name),
                    fields: $fields,
                }
            ).unwrap()
        )
    }
}
