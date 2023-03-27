// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

pub mod call;
mod capabilities;
mod channel;
mod codec;
mod connection;
mod dispatch;
mod message;
pub mod server;
pub mod session;

pub use message::{Action, Object, Service};

pub use qi_format as format;
pub use qi_types as types;
