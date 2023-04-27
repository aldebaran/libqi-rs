// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

pub mod capabilities;
// mod channel;
// pub mod client;
mod message;
pub mod server;
pub mod service;
// pub mod session;

pub use qi_format as format;
pub use qi_types as types;
