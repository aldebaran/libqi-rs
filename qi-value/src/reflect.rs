use crate::{Signature, Type};

pub trait Reflect {
    fn ty() -> Option<Type>;

    fn signature() -> Signature {
        Signature(Self::ty())
    }
}

pub trait RuntimeReflect {
    fn ty(&self) -> Type;

    fn signature(&self) -> Signature {
        Signature(Some(self.ty()))
    }
}
