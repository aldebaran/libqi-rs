use crate::object::error::CallError;
use futures::future::BoxFuture;
use qi_value::{FromValue, Reflect, Type, Value};
use sealed::sealed;
use seq_macro::seq;
use std::any::Any;

#[sealed]
pub trait Method<Args> {
    fn parameter_types() -> Vec<Option<Type>>;
    fn return_type() -> Option<Type>;
    fn boxed(self) -> BoxMethod;
}

macro_rules! impl_fnmut_method {
    (@ $arity:literal) => {
        seq!(N in 0..$arity {
            #[sealed]
            impl<F, #(A~N,)* Ret> Method<(#(A~N,)*)> for F
            where
                F: FnMut(#(A~N,)*) -> Ret + Send + 'static,
                #(
                    for<'v> A~N: Reflect + FromValue<'v> + Send,
                )*
            {
                fn parameter_types() -> Vec<Option<Type>> {
                    vec![
                        #(
                            A~N::ty(),
                        )*
                    ]
                }

                fn return_type() -> Option<Type> {
                    todo!()
                }

                fn boxed(mut self) -> BoxMethod {
                    use futures::future::{self, TryFutureExt};
                    Box::new(move |_dyn_obj, input| {
                        let f = &mut self;
                        let (#(a~N,)*) = match input.cast() {
                            Ok(value) => value,
                            Err(e) => return future::err(CallError::Other(e.into())).boxed(),
                        };
                        f(#(a~N,)*).ok_into().boxed()
                    })
                }
            }
        });
    };
    ($len:literal) => {
        seq!(N in 0..=$len {
            impl_fnmut_method!(@ N);
        });
    }
}

impl_fnmut_method!(16);

struct ReturnValue<T>(T);

pub(super) type BoxMethod = Box<
    dyn FnMut(&mut dyn Any, Value<'_>) -> BoxFuture<'static, Result<Value<'static>, CallError>>
        + Send,
>;
