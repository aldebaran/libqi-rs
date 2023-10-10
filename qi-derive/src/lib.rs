mod typed;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_derive(Typed, attributes(qi))]
pub fn proc_macro_derive_typed(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    typed::derive(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
