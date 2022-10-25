use super::Type;
use seq_macro::seq;
use std::{
    collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque},
    hash::{BuildHasher, Hash},
};

// TODO: This is a bad approach because it forces users to implement ToType.
//       This can create an inconsistency between the Serialize and the ToType
//       implementations for a type.
//       Instead, the signature of a type should be deduced from its Serialize
//       implementation.
//       T => Value => type()
pub trait ToType {
    fn to_type() -> Type;
}

macro_rules! impl_to_type {
    ($ty:ty, $val:expr) => {
        impl ToType for $ty {
            #[inline]
            fn to_type() -> Type {
                $val
            }
        }
    };
}

impl_to_type!((), Type::Void);
impl_to_type!(bool, Type::Bool);
impl_to_type!(i8, Type::Int8);
impl_to_type!(i16, Type::Int16);
impl_to_type!(i32, Type::Int32);
impl_to_type!(i64, Type::Int64);
impl_to_type!(u8, Type::UInt8);
impl_to_type!(u16, Type::UInt16);
impl_to_type!(u32, Type::UInt32);
impl_to_type!(u64, Type::UInt64);
impl_to_type!(isize, Type::Int64);
impl_to_type!(usize, Type::UInt64);
impl_to_type!(f32, Type::Float);
impl_to_type!(f64, Type::Double);
impl_to_type!(char, Type::String);
impl_to_type!(str, Type::String);
impl_to_type!(String, Type::String);
impl_to_type!(std::ffi::CStr, Type::String);
impl_to_type!(std::ffi::CString, Type::String);

impl<T> ToType for Option<T>
where
    T: ToType,
{
    #[inline]
    fn to_type() -> Type {
        Type::option(T::to_type())
    }
}

impl<T, const N: usize> ToType for [T; N]
where
    T: ToType,
{
    #[inline]
    fn to_type() -> Type {
        Type::tuple(vec![T::to_type(); N])
    }
}

macro_rules! impl_list_to_type {
    ($ty:ident < T $(: $tbound1:ident $(+ $tbound2:ident)*)* $(, $typaram:ident : $bound:ident)* >) => {
        impl<T $(, $typaram)*> ToType for $ty<T $(, $typaram)*>
        where
            T: ToType $(+ $tbound1 $(+ $tbound2)*)*,
            $($typaram: $bound,)*
        {
            #[inline]
            fn to_type() -> Type {
                Type::list(T::to_type())
            }
        }
    };
}
impl_list_to_type!(BinaryHeap<T: Ord>);
impl_list_to_type!(BTreeSet<T: Ord>);
impl_list_to_type!(HashSet<T: Eq + Hash, H: BuildHasher>);
impl_list_to_type!(LinkedList<T>);
impl_list_to_type!(Vec<T>);
impl_list_to_type!(VecDeque<T>);

macro_rules! impl_tuples_to_type {
    ($size:literal) => {
        seq!(N in 1..=$size {
            #(
                seq!(I in 0..N {
                    impl< #( T~I, )* > ToType for ( #( T~I, )* )
                    where #( T~I: ToType, )*
                    {
                        #[inline]
                        fn to_type() -> Type {
                            Type::tuple([#(T~I::to_type(),)*])
                        }
                    }
                });
            )*
        });
    };
}
impl_tuples_to_type!(32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_type() {
        assert_eq!(<()>::to_type(), Type::Void);
        assert_eq!(i8::to_type(), Type::Int8);
        assert_eq!(u8::to_type(), Type::UInt8);
        assert_eq!(i16::to_type(), Type::Int16);
        assert_eq!(u16::to_type(), Type::UInt16);
        assert_eq!(i32::to_type(), Type::Int32);
        assert_eq!(u32::to_type(), Type::UInt32);
        assert_eq!(i64::to_type(), Type::Int64);
        assert_eq!(u64::to_type(), Type::UInt64);
        assert_eq!(isize::to_type(), Type::Int64);
        assert_eq!(usize::to_type(), Type::UInt64);
        assert_eq!(f32::to_type(), Type::Float);
        assert_eq!(f64::to_type(), Type::Double);
        assert_eq!(str::to_type(), Type::String);
        assert_eq!(String::to_type(), Type::String);
        assert_eq!(std::ffi::CStr::to_type(), Type::String);
        assert_eq!(std::ffi::CString::to_type(), Type::String);
        todo!("raw");
        // TODO: Objects
        todo!("dynamic");
        assert_eq!(
            Option::<i32>::to_type(),
            Type::Option(Box::new(Type::Int32))
        );
        assert_eq!(<[i32; 0]>::to_type(), Type::tuple([]));
        assert_eq!(<[i32; 1]>::to_type(), Type::tuple([Type::Int32]));
        assert_eq!(<[i32; 10]>::to_type(), Type::tuple(vec![Type::Int32; 10]));
        assert_eq!(
            BinaryHeap::<i32>::to_type(),
            Type::List(Box::new(Type::Int32))
        );
        assert_eq!(
            BTreeSet::<i32>::to_type(),
            Type::List(Box::new(Type::Int32))
        );
        assert_eq!(HashSet::<i32>::to_type(), Type::List(Box::new(Type::Int32)));
        assert_eq!(
            LinkedList::<i32>::to_type(),
            Type::List(Box::new(Type::Int32))
        );
        assert_eq!(Vec::<i32>::to_type(), Type::List(Box::new(Type::Int32)));
        assert_eq!(
            VecDeque::<i32>::to_type(),
            Type::List(Box::new(Type::Int32))
        );
        todo!("map");
        assert_eq!(
            <(i32, u32, i64, u64)>::to_type(),
            Type::tuple([Type::Int32, Type::UInt32, Type::Int64, Type::UInt64])
        );
    }
}
