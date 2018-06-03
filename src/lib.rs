//! `tokio-coap` is a CoAP protocol implementaion
//! that provides an implementaion of the protocol
//! for use with`tokio-core`.

extern crate futures;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_dns;
extern crate bytes;
extern crate smallvec;
#[macro_use]
extern crate log;
extern crate uri;
extern crate percent_encoding;

pub mod client;
pub mod codec;
pub mod endpoint;
pub mod error;
pub mod message;

pub use client::Client;
pub use endpoint::Endpoint;
