// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

pub mod capabilities;
mod channel;
mod client;
mod control;
mod message;
pub mod request;
mod server;
// pub mod session;

pub use qi_format as format;
pub use qi_types as types;
