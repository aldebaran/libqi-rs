mod value;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};
use value::{derive_impl, Derive};

#[proc_macro_derive(Reflect, attributes(qi))]
pub fn proc_macro_derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Derive::Reflect, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(AsValue, attributes(qi))]
pub fn proc_macro_derive_as_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Derive::AsValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(FromValue, attributes(qi))]
pub fn proc_macro_derive_from_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Derive::FromValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(StdTryFromValue, attributes(qi))]
pub fn proc_macro_derive_std_try_from_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Derive::StdTryFromValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(IntoValue, attributes(qi))]
pub fn proc_macro_derive_into_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_impl(Derive::IntoValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
