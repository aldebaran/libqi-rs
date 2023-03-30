// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

pub mod call;
mod capabilities;
mod channel;
mod codec;
pub mod connection;
mod dispatch;
mod message;
pub mod server;
pub mod session;

pub use connection::Connection;
pub use message::{Action, Object, Recipient, Service};
pub use session::Session;

pub use qi_format as format;
pub use qi_types as types;
