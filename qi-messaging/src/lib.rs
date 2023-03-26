// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

mod capabilities;
pub mod client;
mod codec;
mod dispatch;
mod message;
mod message_types;
pub mod server;
mod session;

pub use message::{Action, Object, Service};
pub use message_types::Response;
pub use session::Session;

pub use qi_format as format;
pub use qi_types as types;
