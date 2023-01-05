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
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
    Tuple {
        elements: Vec<Type>,
        annotations: Option<Annotations>,
    },
}

impl Type {
    pub fn common_type(&self, _t: &Type) -> Option<Type> {
        todo!()
    }
}

impl Default for Type {
    fn default() -> Self {
        Self::Unit
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Annotations {
    pub name: String,
    pub fields: Vec<String>,
}
