#![allow(clippy::wrong_self_convention)]
mod object;
mod value;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_derive(Valuable, attributes(qi))]
pub fn proc_macro_derive_valuable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    value::derive_impl(value::Trait::Valuable, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Reflect, attributes(qi))]
pub fn proc_macro_derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    value::derive_impl(value::Trait::Reflect, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(ToValue, attributes(qi))]
pub fn proc_macro_derive_to_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    value::derive_impl(value::Trait::ToValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(IntoValue, attributes(qi))]
pub fn proc_macro_derive_into_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    value::derive_impl(value::Trait::IntoValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(FromValue, attributes(qi))]
pub fn proc_macro_derive_from_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    value::derive_impl(value::Trait::FromValue, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Declares an object type.
///
/// Checks `#[qi::method]`, `#[qi::property]`, and `#[qi::signal]` members,
/// emits the trait with those attributes removed, defines
/// `<TRAIT>_META_OBJECT` as `once_cell::sync::Lazy<qi::object::MetaObject>`,
/// and implements `qi::Object` for types that implement the trait.
///
/// `#[qi::object(client)]` implements `Object` only for the generated client,
/// by delegating to its `ObjectClient`. A crate can emit only one blanket
/// `impl<T: Trait> Object for T`.
///
/// # Example
///
/// ```ignore
/// #[qi::object]
/// trait Motion {
///     /// Go to some position.
///     #[qi::method(name = "goTo")]
///     async fn go_to(&self, position: Position) -> Result<(), Error>;
///
///     /// The current position.
///     #[qi::property]
///     fn position(&self) -> qi::Property<Position>;
///
///     /// The moving state.
///     #[qi::signal]
///     fn moving(&self) -> qi::Signal<bool>;
/// }
///
/// #[derive(qi::Valuable)]
/// #[qi(value(crate = "qi_value"))]
/// struct Position {
///     x: u32,
///     y: u32,
/// }
/// ```
#[proc_macro_attribute]
pub fn object(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut parsed = parse_macro_input!(item as object::Object);
    parsed.mode = parse_macro_input!(attr as object::ObjectMode);
    parsed.to_token_stream().into()
}
