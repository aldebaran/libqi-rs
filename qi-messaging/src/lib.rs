// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

pub mod capabilities;
mod channel;
mod client;
mod format;
mod message;
mod request;
mod server;
pub mod session;

pub use session::{ClientError, Request, Session};
