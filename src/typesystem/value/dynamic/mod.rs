mod de;
mod ser;
pub use de::from_dynamic;
pub use ser::to_dynamic;

// TODO: #[non_exhaustive]
// TODO: Enable the value to borrow data from sources.
// TODO: This is a dynamic value, and should be de/serialized as such.
#[derive(Default, Clone, PartialEq, PartialOrd, Debug)]
pub enum Dynamic {
    #[default]
    Void,
    Bool(bool),
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float(f32),
    Double(f64),
    String(String),
    Raw(Vec<u8>),
    Optional(Option<Box<Dynamic>>),
    List(Vec<Dynamic>),
    Map(Vec<(Dynamic, Dynamic)>),
    Tuple(Tuple),
    // TODO: Handle enumerations
}

pub mod tuple {
    use super::Dynamic;
    use crate::typesystem::tuple;
    pub type Tuple = tuple::Tuple<Dynamic>;
    pub type Elements = tuple::Elements<Dynamic>;
    pub type Field = tuple::Field<Dynamic>;
}
pub use tuple::Tuple;

impl Dynamic {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            Dynamic::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_string().map(|s| s.as_str())
    }

    pub fn as_tuple(&self) -> Option<&Tuple> {
        if let Self::Tuple(tuple) = self {
            Some(tuple)
        } else {
            None
        }
    }

    pub fn as_tuple_mut(&mut self) -> Option<&mut Tuple> {
        if let Self::Tuple(tuple) = self {
            Some(tuple)
        } else {
            None
        }
    }
}

impl From<String> for Dynamic {
    fn from(s: String) -> Self {
        Dynamic::String(s)
    }
}

impl TryFrom<Dynamic> for String {
    type Error = TryFromDynamicError;
    fn try_from(d: Dynamic) -> Result<Self, Self::Error> {
        match d {
            Dynamic::String(s) => Ok(s),
            _ => Err(TryFromDynamicError),
        }
    }
}

impl From<&str> for Dynamic {
    fn from(s: &str) -> Self {
        Dynamic::String(s.into())
    }
}

impl<'v> TryFrom<&'v Dynamic> for &'v str {
    type Error = TryFromDynamicError;
    fn try_from(d: &'v Dynamic) -> Result<Self, Self::Error> {
        d.as_str().ok_or(TryFromDynamicError)
    }
}

// TODO: Implement all conversions

impl From<Tuple> for Dynamic {
    fn from(t: Tuple) -> Self {
        Dynamic::Tuple(t)
    }
}

impl TryFrom<Dynamic> for Tuple {
    type Error = TryFromDynamicError;

    fn try_from(d: Dynamic) -> Result<Self, Self::Error> {
        match d {
            Dynamic::Tuple(t) => Ok(t),
            _ => Err(TryFromDynamicError),
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[error("dynamic conversion failed")]
pub struct TryFromDynamicError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::tests::Serializable;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_dynamic_from_string() {
        assert_eq!(
            Dynamic::from("muffins recipe".to_owned()),
            Dynamic::String("muffins recipe".into())
        );
    }

    #[test]
    fn test_dynamic_try_into_string() {
        let res: Result<String, _> = Dynamic::String("muffins recipe".into()).try_into();
        assert_eq!(res, Ok("muffins recipe".to_owned()));
        let res: Result<String, _> = Dynamic::Int32(321).try_into();
        assert_eq!(res, Err(TryFromDynamicError));
    }

    #[test]
    fn test_dynamic_from_str() {
        assert_eq!(
            Dynamic::from("cookies recipe"),
            Dynamic::String("cookies recipe".into())
        );
    }

    #[test]
    fn test_dynamic_try_into_str() {
        let value = Dynamic::String("muffins recipe".into());
        let res: Result<&str, _> = (&value).try_into();
        assert_eq!(res, Ok("muffins recipe"));
        let res: Result<&str, _> = (&Dynamic::Int32(321)).try_into();
        assert_eq!(res, Err(TryFromDynamicError));
    }

    #[test]
    fn test_dynamic_as_string() {
        assert_eq!(
            Dynamic::from("muffins").as_string(),
            Some(&"muffins".to_owned())
        );
        assert_eq!(Dynamic::Int32(321).as_string(), None);
    }

    #[test]
    fn test_dynamic_as_str() {
        assert_eq!(Dynamic::from("cupcakes").as_str(), Some("cupcakes"));
        assert_eq!(Dynamic::Float(3.14).as_str(), None);
    }

    #[test]
    fn test_dynamic_from_tuple() {
        assert_eq!(
            Dynamic::from(Tuple::default()),
            Dynamic::Tuple(Tuple {
                name: Default::default(),
                elements: Default::default()
            }),
        );
    }

    #[test]
    fn test_dynamic_try_into_tuple() {
        let t: Result<Tuple, _> = Dynamic::Tuple(Tuple {
            name: Default::default(),
            elements: Default::default(),
        })
        .try_into();
        assert_eq!(t, Ok(Tuple::default()));
        let t: Result<Tuple, _> = Dynamic::from("cheesecake").try_into();
        assert_eq!(t, Err(TryFromDynamicError));
    }

    #[test]
    fn test_dynamic_as_tuple() {
        assert_eq!(
            Dynamic::Tuple(Default::default()).as_tuple(),
            Some(&Tuple::default())
        );
        assert_eq!(Dynamic::Int32(42).as_tuple(), None);
    }

    #[test]
    fn test_dynamic_as_tuple_mut() {
        assert_eq!(
            Dynamic::Tuple(Default::default()).as_tuple_mut(),
            Some(&mut Tuple::default())
        );
        assert_eq!(Dynamic::Int32(42).as_tuple_mut(), None);
    }

    #[test]
    fn test_to_dynamic() {
        let (s, expected) = crate::tests::sample_serializable_and_dynamic_value();
        let dynamic = to_dynamic(&s).unwrap();
        assert_eq!(dynamic, expected);
    }

    #[test]
    fn test_from_dynamic() {
        let (expected, v) = crate::tests::sample_serializable_and_dynamic_value();
        let s: Serializable = from_dynamic(v).unwrap();
        assert_eq!(s, expected);
    }

    #[test]
    fn test_to_from_dynamic_invariant() -> Result<(), ser::Error> {
        let (s, _) = crate::tests::sample_serializable_and_dynamic_value();
        let s2: Serializable = from_dynamic(to_dynamic(&s)?)?;
        assert_eq!(s, s2);
        Ok(())
    }
}
