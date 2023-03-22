// TODO: #![deny(missing_docs)]
// Deny warnings in doc test.
#![doc(test(attr(deny(warnings))))]
#![doc = include_str!("../README.md")]

mod capabilities;
pub mod channel;
mod message;
mod stream;

#[doc(inline)]
pub use capabilities::CapabilityMap;

#[doc(inline)]
pub use message::Message;

#[doc(inline)]
pub use channel::Channel;
