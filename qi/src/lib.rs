#![deny(unreachable_pub, unsafe_code)]
#![warn(
    clippy::all,
    clippy::clone_on_ref_ptr,
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::empty_drop,
    clippy::empty_structs_with_brackets,
    clippy::exit,
    clippy::float_cmp_const,
    clippy::format_push_string,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::integer_division,
    clippy::large_include_file,
    clippy::let_underscore_must_use,
    clippy::lossy_float_literal,
    clippy::map_err_ignore,
    clippy::mem_forget,
    clippy::mixed_read_write_in_expression,
    clippy::multiple_inherent_impl,
    clippy::mutex_atomic,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_name_method,
    clippy::mod_module_files,
    clippy::str_to_string,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::todo,
    clippy::try_err,
    clippy::unimplemented,
    clippy::unnecessary_self_imports,
    clippy::unneeded_field_pattern,
    clippy::use_debug
)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

mod error;
mod never;
pub mod node;
pub mod object;
pub mod service;
pub(crate) mod service_directory;
pub mod session;
pub mod signal;
pub mod value;

pub use self::{
    error::{BoxError, Error, HandlerError},
    never::Never,
    node::Node,
    object::{Object, ObjectExt},
    service_directory::ServiceDirectory,
    value::Value,
};
pub use qi_format as format;
pub use qi_macros::{object, FromValue, IntoValue, Reflect, ToValue, Valuable};
pub use qi_messaging::{self as messaging, Address};

pub type Result<T> = std::result::Result<T, Error>;
