/// The type of a value in the `qi` type system.
///
/// The absence of a type means a value is dynamic, i.e. its type can change at runtime.
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
    Tuple(TupleType),
}

impl Type {
    pub(crate) fn is_convertible_to(&self, target: &Type) -> bool {
        match (self, target) {
            (Type::Option(source), Type::Option(target)) => {
                is_convertible_to(source.as_deref(), target.as_deref())
            }
            (Type::List(source), Type::List(target)) => {
                is_convertible_to(source.as_deref(), target.as_deref())
            }
            (
                Type::Map {
                    key: source_key,
                    value: source_value,
                },
                Type::Map {
                    key: target_key,
                    value: target_value,
                },
            ) => {
                is_convertible_to(source_key.as_deref(), target_key.as_deref())
                    && is_convertible_to(source_value.as_deref(), target_value.as_deref())
            }
            (Type::Tuple(source), Type::Tuple(target)) => source.is_convertible_to(target),
            (source, target) => source == target,
        }
    }
}

/// Defaults constructs a type as a unit type.
impl Default for Type {
    fn default() -> Self {
        Self::Unit
    }
}

impl From<TupleType> for Type {
    fn from(tuple: TupleType) -> Self {
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
                write_option_type(f, t.as_deref())?;
                f.write_str(")")
            }
            Type::List(t) => {
                f.write_str("list(")?;
                write_option_type(f, t.as_deref())?;
                f.write_str(")")
            }
            Type::VarArgs(t) => {
                f.write_str("varargs(")?;
                write_option_type(f, t.as_deref())?;
                f.write_str(")")
            }
            Type::Map { key, value } => {
                f.write_str("map(")?;
                write_option_type(f, key.as_deref())?;
                f.write_str(",")?;
                write_option_type(f, value.as_deref())?;
                f.write_str(")")
            }
            Type::Tuple(t) => t.fmt(f),
        }
    }
}

pub(crate) fn common_type(t1: Option<Type>, t2: Option<Type>) -> Option<Type> {
    match (t1, t2) {
        (Some(t1), Some(t2)) if t1 == t2 => Some(t1),
        _ => None,
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TupleType {
    Tuple(Vec<Option<Type>>),
    TupleStruct(String, Vec<Option<Type>>),
    Struct(String, Vec<StructField>),
}

impl TupleType {
    pub fn new() -> Self {
        Self::Tuple(vec![])
    }

    pub fn from_annotations_of_elements(
        annotations: StructAnnotations,
        elements: Vec<Option<Type>>,
    ) -> Result<Self, ZipStructFieldsSizeError> {
        let tuple = if let Some(field_names) = annotations.field_names {
            TupleType::Struct(annotations.name, zip_struct_fields(field_names, elements)?)
        } else {
            TupleType::TupleStruct(annotations.name, elements)
        };
        Ok(tuple)
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Tuple(t) => t.len(),
            Self::TupleStruct(_, t) => t.len(),
            Self::Struct(_, s) => s.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Tuple(t) => t.is_empty(),
            Self::TupleStruct(_, t) => t.is_empty(),
            Self::Struct(_, s) => s.is_empty(),
        }
    }

    pub fn element_types(&self) -> Vec<Option<Type>> {
        match self {
            Self::Tuple(t) => t.clone(),
            Self::TupleStruct(_, t) => t.clone(),
            Self::Struct(_, s) => s.iter().map(|field| field.value_type.clone()).collect(),
        }
    }

    pub fn name(&self) -> Option<String> {
        match self {
            Self::Tuple(_) => None,
            Self::TupleStruct(name, _) | Self::Struct(name, _) => Some(name.clone()),
        }
    }

    pub fn field_names(&self) -> Option<Vec<String>> {
        match self {
            Self::Tuple(_) | Self::TupleStruct(_, _) => None,
            Self::Struct(_, this) => Some(this.iter().map(|field| field.name.clone()).collect()),
        }
    }

    pub fn annotations(&self) -> Option<StructAnnotations> {
        match self {
            Self::Tuple(_) => None,
            Self::TupleStruct(name, _) => Some(StructAnnotations {
                name: name.clone(),
                field_names: None,
            }),
            Self::Struct(name, fields) => Some(StructAnnotations {
                name: name.clone(),
                field_names: Some(fields.iter().map(|field| field.name.clone()).collect()),
            }),
        }
    }

    /// Tuple conversion is defined as follows:
    /// - the size of `self` must match the size of the target,
    /// - then if both `self` and `target` have a name, they must match,
    /// - then if both `self` have field names, they must match, in order.
    fn is_convertible_to(&self, target: &TupleType) -> bool {
        self.len() == target.len()
            && match (self.name(), target.name()) {
                (Some(name), Some(target_name)) => name == target_name,
                _ => true,
            }
            && match (self.field_names(), target.field_names()) {
                (Some(field_names), Some(target_field_names)) => field_names == target_field_names,
                _ => true,
            }
    }
}

impl Default for TupleType {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TupleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("tuple(")?;
        for (idx, element) in self.element_types().into_iter().enumerate() {
            if idx > 0 {
                f.write_str(",")?;
            }
            write_option_type(f, element.as_ref())?;
        }
        f.write_str(")")?;
        if let Some(annotations) = self.annotations() {
            annotations.fmt(f)?;
        }
        Ok(())
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct StructField {
    pub name: String,
    pub value_type: Option<Type>,
}

impl From<(String, Option<Type>)> for StructField {
    fn from(v: (String, Option<Type>)) -> Self {
        Self {
            name: v.0,
            value_type: v.1,
        }
    }
}

pub fn zip_struct_fields<N, E>(
    names: N,
    elements: E,
) -> Result<Vec<StructField>, ZipStructFieldsSizeError>
where
    N: IntoIterator,
    N::Item: Into<String>,
    E: IntoIterator,
    E::Item: Into<Option<Type>>,
{
    let mut names = names.into_iter();
    let mut elements = elements.into_iter();
    let mut fields = Vec::new();
    for count in 0.. {
        match (names.next(), elements.next()) {
            (Some(name), Some(element)) => fields.push(StructField {
                name: name.into(),
                value_type: element.into(),
            }),
            (Some(_), None) => {
                return Err(ZipStructFieldsSizeError {
                    name_count: count + 1,
                    element_count: count,
                })
            }
            (None, Some(_)) => {
                return Err(ZipStructFieldsSizeError {
                    name_count: count,
                    element_count: count + 1,
                })
            }
            (None, None) => break,
        }
    }
    Ok(fields)
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, thiserror::Error)]
#[error(
    "zip of structure fields error of sizes, got {element_count} elements for {name_count} names"
)]
pub struct ZipStructFieldsSizeError {
    pub name_count: usize,
    pub element_count: usize,
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

pub fn option_of<T>(t: T) -> Type
where
    T: Into<Option<Type>>,
{
    Type::Option(t.into().map(Box::new))
}

#[cfg(test)]
pub fn varargs_of<T>(t: T) -> Type
where
    T: Into<Option<Type>>,
{
    Type::VarArgs(t.into().map(Box::new))
}

pub fn list_of<T>(t: T) -> Type
where
    T: Into<Option<Type>>,
{
    Type::List(t.into().map(Box::new))
}

pub fn map_of<K, V>(key: K, value: V) -> Type
where
    K: Into<Option<Type>>,
    V: Into<Option<Type>>,
{
    Type::Map {
        key: key.into().map(Box::new),
        value: value.into().map(Box::new),
    }
}

#[macro_export]
macro_rules! ty {
    (Unit) => {
        $crate::ty::Type::Unit
    };
    (Bool) => {
        $crate::ty::Type::Bool
    };
    (Int8) => {
        $crate::ty::Type::Int8
    };
    (UInt8) => {
        $crate::ty::Type::UInt8
    };
    (Int16) => {
        $crate::ty::Type::Int16
    };
    (UInt16) => {
        $crate::ty::Type::UInt16
    };
    (Int32) => {
        $crate::ty::Type::Int32
    };
    (UInt32) => {
        $crate::ty::Type::UInt32
    };
    (Int64) => {
        $crate::ty::Type::Int64
    };
    (UInt64) => {
        $crate::ty::Type::UInt64
    };
    (Float32) => {
        $crate::ty::Type::Float32
    };
    (Float64) => {
        $crate::ty::Type::Float64
    };
    (String) => {
        $crate::ty::Type::String
    };
    (Raw) => {
        $crate::ty::Type::Raw
    };
    (Object) => {
        $crate::ty::Type::Object
    };
    ($t:expr) => {
        $crate::ty::Type::from($t)
    };
}

#[macro_export]
macro_rules! option_ty {
    ($t:expr) => {
        $crate::ty::option_of($t)
    };
}

#[macro_export]
macro_rules! list_ty {
    ($t:expr) => {
        $crate::ty::list_of($t)
    };
}

#[macro_export]
macro_rules! varargs_ty {
    ($t:expr) => {
        $crate::ty::varargs_of($t)
    };
}

#[macro_export]
macro_rules! map_ty {
    ($key:expr , $value:expr) => {
        $crate::ty::map_of($key, $value)
    };
}

#[macro_export]
macro_rules! tuple_ty {
    ($($t:expr),+ $(,)*) => {
        $crate::ty::Type::Tuple(
            $crate::ty::TupleType::Tuple(
                vec![$($t.into()),+]
            )
        )
    };
    () => {
        $crate::ty::Type::Tuple(
            $crate::ty::TupleType::new()
        )
    };
}

#[macro_export]
macro_rules! struct_ty {
    ($name:ident ( $($t:expr),* $(,)* )) => {
        $crate::ty::Type::Tuple(
            $crate::ty::TupleType::TupleStruct(
                stringify!($name).to_string(),
                vec![$($t.into()),*],
            )
        )
    };
    ($name:ident { $($f:ident : $t:expr),* $(,)* }) => {
        $crate::ty::Type::Tuple(
            $crate::ty::TupleType::Struct(
                stringify!($name).to_string(),
                vec![
                    $(
                        $crate::ty::StructField {
                            name: stringify!($f).to_string(),
                            value_type: $t.into(),
                        }
                    ),*
                ],
            )
        )
    };
}

/// Trait for types that can be statically reflected on.
pub trait StaticGetType {
    fn get_type() -> Type;
}

/// Trait for types that can be dynamically reflected on.
pub trait DynamicGetType {
    fn get_type(&self) -> Type;

    fn is_assignable_to(&self, t: &Type) -> bool {
        self.get_type().is_convertible_to(t)
    }
}

/// A statically typed value is also dynamically typed.
impl<T> DynamicGetType for T
where
    T: StaticGetType,
{
    fn get_type(&self) -> Type {
        T::get_type()
    }
}

fn is_convertible_to(source: Option<&Type>, target: Option<&Type>) -> bool {
    match (source, target) {
        // No target type information, conversion is allowed.
        (_, None) => true,
        // No source type information, conversion is not allowed.
        (None, Some(_)) => false,
        // Both source and target type information, check for standard conversion.
        (Some(source), Some(target)) => source.is_convertible_to(target),
    }
}

fn write_option_type(f: &mut std::fmt::Formatter<'_>, t: Option<&Type>) -> std::fmt::Result {
    use std::fmt::Display;
    match t {
        Some(t) => t.fmt(f),
        None => f.write_str("dynamic"),
    }
}
