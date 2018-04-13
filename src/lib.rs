//! `tokio-coap` is a CoAP protocol implementaion
//! that provides an implementaion of the protocol
//! for use with`tokio-core`.

extern crate futures;
extern crate tokio;
extern crate tokio_io;
extern crate bytes;
extern crate smallvec;

pub mod codec;
pub mod message;
