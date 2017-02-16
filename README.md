Tokio-CoAP
==========

[![Build Status](https://travis-ci.org/azdle/tokio-coap.svg?branch=master)](https://travis-ci.org/azdle/tokio-coap)
[![Crates.io Link](http://meritbadge.herokuapp.com/tokio-coap)](https://crates.io/crates/tokio-coap)

tokio-coap is a [CoAP](https://tools.ietf.org/html/rfc7252) protocol
implementation built on top of the [tokio platform](https://tokio.rs/)
in the [Rust programming language](https://rust-lang.org).

Because tokio-coap is written using tokio for all network requests, it should
provide very low overheads, making it possible to use as part of very fast and
efficient systems.

Status
------

**tokio-coap is incomplete and you likely shouldn't use it yet**

Currently it is possible to create servers that directly deal with incoming
CoAP packets, but there is not yet any automatic handling of retries or
multi-packet messages.

There is not yet any implementation of client requests.

No benchmarks have been done to determine if it, in it's current state, is as
fast as it could be.


Getting Started
---------------

There is very little in the way of useful documentation at the moment, for now
check out the rst-all.rs example for how to use th library.

This example shows how to write the simplest possible message handler. It
simply replies to any valid CoAP packets that would expect a response with a
RST message.
