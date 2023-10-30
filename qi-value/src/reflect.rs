use crate::{Signature, Type};

pub trait Reflect {
    fn ty() -> Option<Type>;

    fn signature() -> Signature {
        Signature(Self::ty())
    }
}
