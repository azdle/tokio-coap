Tokio-CoAP
==========

[![Build Status](https://travis-ci.org/azdle/tokio-coap.svg?branch=master)](https://travis-ci.org/azdle/tokio-coap)
[![Crates.io Link](http://meritbadge.herokuapp.com/tokio-coap)](https://crates.io/crates/tokio-coap)

`tokio-coap` is a [CoAP](https://tools.ietf.org/html/rfc7252) protocol
implementation built on top of the [tokio platform](https://tokio.rs/)
in the [Rust programming language](https://rust-lang.org).

`tokio-coap`, by the of the nature of being written on top of tokio, it built
on top of [`futures`](https://github.com/alexcrichton/futures-rs) making it
ideal for high performance and/or low resource use cases.

Status
------

**tokio-coap is incomplete and you likely shouldn't use it yet**

`tokio-coap` is still taking shape, feedback on the interface is extremely
welcome. Please open an issue on github if you have interest is using this
library and want to provide and feedback about issues you have with the API,
features you'd like to see prioritized, or even just general use cases you'd
like to make sure are supported in the future.

Currently it is possible to directly deal with CoAP packets, but there is not
yet any notion of a client or server (or more like an "endpoint" that is both)
or any handling of sending or reciving packet on the network.

There are basic benchmarks to judge how changes affect performance, but no
optomizing has been done explicitly yet. This will come in the future, but will
happen after a 1.0.0 release.


Getting Started
---------------

There is very little in the way of useful documentation at the moment, for now
check out the various examples for how to use the library.

