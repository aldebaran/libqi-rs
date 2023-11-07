#[allow(clippy::wrong_self_convention)]
mod value;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};
use value::{derive_impl, Trait};

#[proc_macro_derive(Reflect, attributes(qi))]
pub fn proc_macro_derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Trait::Reflect, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(ToValue, attributes(qi))]
pub fn proc_macro_derive_to_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Trait::ToValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(IntoValue, attributes(qi))]
pub fn proc_macro_derive_into_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Trait::IntoValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(FromValue, attributes(qi))]
pub fn proc_macro_derive_from_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Trait::FromValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
