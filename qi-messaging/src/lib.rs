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

// TODO: remove for public, currently only available because of benchmarks.
pub use codec::MessageCodec;
pub use connection::{Connection, ConnectionError};
pub use message::{Action, Flags, Id as MessageId, Message, Object, Payload, Service, Type};
pub use session::Session;

pub use qi_format as format;
pub use qi_types as types;
