#![deny(unreachable_pub, unsafe_code)]
// TODO: #![deny(missing_docs)]
#![warn(unused_crate_dependencies)]
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
    clippy::panic,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_name_method,
    clippy::self_named_module_files,
    clippy::single_char_lifetime_names,
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

mod capabilities;
mod channel;
mod client;
mod message;
mod request;
mod server;
pub mod session;

use qi_format as format;
