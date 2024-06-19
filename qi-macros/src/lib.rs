#![allow(clippy::wrong_self_convention)]
mod object;
mod value;

use proc_macro::TokenStream;
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
/// This macro declares a new trait and implements `qi::Object` for any
/// type that implements that trait. It also declares a client type that
/// implements that trait to call the object interface remotely.
///
/// # Example
///
/// ```
/// # mod qi {
/// #   pub(super) use qi_macros::{object, Valuable};
/// # }
/// #[qi::object]
/// trait Motion {
///     /// Go to some position.
///     async fn go_to(&self, position: Position) -> Result<(), Error>;
///
///     /// The current position.
///     #[property]
///     fn position() -> Position;
///
///     /// The moving state.
///     #[signal]
///     fn moving() -> bool;
/// }
///
/// #[derive(qi::Valuable)]
/// ##[qi(value(crate = "qi_value"))]
/// struct Position {
///     x: u32,
///     y: u32,
/// }
/// ```
///
/// This code declares the trait `Motion` and and a type `MotionClient`
/// that implements `Motion`.
#[proc_macro_attribute]
pub fn object(_attr: TokenStream, _item: TokenStream) -> TokenStream {
    todo!()
    // let input = parse_macro_input!(input as object::Object);
    // input.derive().into()
}
