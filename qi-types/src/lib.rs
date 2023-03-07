mod num_bool;
#[doc(inline)]
pub use num_bool::{
    Bool, Float32, Float64, Int16, Int32, Int64, Int8, Number, UInt16, UInt32, UInt64, UInt8,
};

mod tuple;
#[doc(inline)]
pub use tuple::{Tuple, Unit};

// The module is not named `type` because it is a keyword.
pub mod typing;
#[doc(inline)]
pub use typing::Type;

mod signature;
#[doc(inline)]
pub use signature::Signature;

pub type Str = str;
pub type String = std::string::String;

pub type Raw = bytes::Bytes;

pub type Option<T> = std::option::Option<T>;

pub type List<T> = std::vec::Vec<T>;

#[macro_export]
macro_rules! list {
    ($($tt:tt)*) => {
        vec![$($tt)*]
    }
}

pub mod map;
#[doc(inline)]
pub use map::Map;

pub mod value;
#[doc(inline)]
pub use value::Value;

pub mod dynamic;
#[doc(inline)]
pub use dynamic::Dynamic;

pub mod object;
#[doc(inline)]
pub use object::{MetaMethod, MetaObject, MetaProperty, MetaSignal, Object};
