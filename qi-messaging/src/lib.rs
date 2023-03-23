// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

mod capabilities;
//pub mod client;
mod message;
mod req_rep;
//pub mod server;
mod session;
mod stream;

pub use session::Session;

pub use qi_format as format;
pub use qi_types as types;
