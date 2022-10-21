#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum Type {
    #[default]
    None,
    Unknown,
    Void,
    Bool,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float,
    Double,
    String,
    Raw,
    Object,
    Dynamic,
    Option(Box<Type>),
    List(Box<Type>),
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
    Tuple(Vec<Type>),
    VarArgs(Box<Type>),
    KwArgs(Box<Type>),
}

impl Type {
    pub fn list<T>(t: T) -> Self
    where
        T: Into<Box<Self>>,
    {
        Self::List(t.into())
    }

    pub fn map<K, V>(key: K, value: V) -> Self
    where
        K: Into<Box<Self>>,
        V: Into<Box<Self>>,
    {
        Self::Map {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn tuple<I>(elems: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        Self::Tuple(elems.into_iter().collect())
    }

    pub fn var_args<T>(t: T) -> Self
    where
        T: Into<Box<Self>>,
    {
        Self::VarArgs(t.into())
    }

    pub fn kw_args<T>(t: T) -> Self
    where
        T: Into<Box<Self>>,
    {
        Self::KwArgs(t.into())
    }

    pub fn option<T>(t: T) -> Self
    where
        T: Into<Box<Self>>,
    {
        Self::Option(t.into())
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        match self {
            Type::None => f.write_char('_'),
            Type::Unknown => f.write_char('X'),
            Type::Void => f.write_char('v'),
            Type::Bool => f.write_char('b'),
            Type::Int8 => f.write_char('c'),
            Type::UInt8 => f.write_char('C'),
            Type::Int16 => f.write_char('w'),
            Type::UInt16 => f.write_char('W'),
            Type::Int32 => f.write_char('i'),
            Type::UInt32 => f.write_char('I'),
            Type::Int64 => f.write_char('l'),
            Type::UInt64 => f.write_char('L'),
            Type::Float => f.write_char('f'),
            Type::Double => f.write_char('d'),
            Type::String => f.write_char('s'),
            Type::Raw => f.write_char('r'),
            Type::Object => f.write_char('o'),
            Type::Dynamic => f.write_char('m'),
            Type::Option(o) => write!(f, "+{o}"),
            Type::List(l) => write!(f, "[{l}]"),
            Type::Map { key, value } => write!(f, "{{{key}{value}}}"),
            Type::Tuple(t) => write!(
                f,
                "({})",
                t.iter().fold(String::new(), |s, t| s + &t.to_string())
            ),
            Type::VarArgs(a) => write!(f, "#{a}"),
            Type::KwArgs(a) => write!(f, "~{a}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_list() {
        assert_eq!(Type::list(Type::String), Type::List(Box::new(Type::String)));
    }

    #[test]
    fn test_type_map() {
        assert_eq!(
            Type::map(Type::String, Type::UInt8,),
            Type::Map {
                key: Box::new(Type::String),
                value: Box::new(Type::UInt8)
            }
        );
    }

    #[test]
    fn test_type_tuple() {
        assert_eq!(
            Type::tuple([Type::Int32, Type::Float, Type::String]),
            Type::Tuple(vec![Type::Int32, Type::Float, Type::String,]),
        );
    }

    #[test]
    fn test_type_var_args() {
        assert_eq!(
            Type::var_args(Type::list(Type::String)),
            Type::VarArgs(Box::new(Type::List(Box::new(Type::String))))
        );
    }

    #[test]
    fn test_type_kw_args() {
        assert_eq!(Type::kw_args(Type::Raw), Type::KwArgs(Box::new(Type::Raw)));
    }

    #[test]
    fn test_type_to_string() {
        let assert_to_string_eq = |t: Type, s: &str| assert_eq!(t.to_string(), s);
        assert_to_string_eq(Type::None, "_");
        assert_to_string_eq(Type::Unknown, "X");
        assert_to_string_eq(Type::Void, "v");
        assert_to_string_eq(Type::Bool, "b");
        assert_to_string_eq(Type::Int8, "c");
        assert_to_string_eq(Type::UInt8, "C");
        assert_to_string_eq(Type::Int16, "w");
        assert_to_string_eq(Type::UInt16, "W");
        assert_to_string_eq(Type::Int32, "i");
        assert_to_string_eq(Type::UInt32, "I");
        assert_to_string_eq(Type::Int64, "l");
        assert_to_string_eq(Type::UInt64, "L");
        assert_to_string_eq(Type::Float, "f");
        assert_to_string_eq(Type::Double, "d");
        assert_to_string_eq(Type::String, "s");
        assert_to_string_eq(Type::Raw, "r");
        assert_to_string_eq(Type::Object, "o");
        assert_to_string_eq(Type::Dynamic, "m");
        assert_to_string_eq(Type::option(Type::Void), "+v");
        assert_to_string_eq(Type::list(Type::Int32), "[i]");
        assert_to_string_eq(Type::map(Type::Float, Type::String), "{fs}");
        assert_to_string_eq(
            Type::tuple([Type::Float, Type::String, Type::UInt32]),
            "(fsI)",
        );
        assert_to_string_eq(Type::var_args(Type::Dynamic), "#m");
        assert_to_string_eq(Type::kw_args(Type::Object), "~o");
        // Some complex type for fun.
        assert_to_string_eq(
            Type::tuple([
                Type::list(Type::map(Type::option(Type::Object), Type::Raw)),
                Type::kw_args(Type::Double),
                Type::var_args(Type::option(Type::Dynamic)),
            ]),
            "([{+or}]~d#+m)",
        )
    }
}
