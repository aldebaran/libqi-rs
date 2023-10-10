mod impls;

use crate::{Signature, Type};

pub trait Typed {
    fn ty() -> Option<Type>;

    fn signature() -> Signature {
        Signature(Self::ty())
    }
}
