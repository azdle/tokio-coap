extern crate tokio_coap;
extern crate futures;
extern crate tokio_core;
extern crate env_logger;

use std::net::SocketAddr;

use futures::{Stream, Sink};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use tokio_coap::message::{Mtype, Code};
use tokio_coap::message::Code::{Content, NotImplemented};
use tokio_coap::message::option::UriPath;

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let addr: SocketAddr = "0.0.0.0:5683".parse().unwrap();

    let sock = UdpSocket::bind(&addr, &handle).unwrap();

    let (sink, stream) = sock.framed(tokio_coap::codec::CoapCodec).split();

    let stream = stream.map(|(addr, req)| {
        println!("--> {:?}", req);

        if let Some(mut req) = req {
            match req.mtype {
                Mtype::Confirmable | Mtype::NonConfirmable => {
                    let path = req.options.get::<UriPath>();
                    match (&req.code, &path) {
                        (&Code::Get, &Some(ref p)) if p == &["ip".into()] => {
                            (addr,
                             Some(req.new_reply()
                                .with_code(Content)
                                .with_payload(addr.ip()
                                                  .to_string()
                                                  .as_bytes()
                                                  .to_owned())))
                        }
                        _ => {
                            (addr, Some(req.new_reply().with_code(NotImplemented)))
                        }
                    }
                }
                _ => {
                    println!("<-X Not replying to message of type: {:?}", req.mtype);
                    (addr, None)
                }
            }
        } else {
            println!("<-X Not replying to invalid message");
            (addr, None)
        }
    });

    let sock = sink.send_all(stream);
    drop(core.run(sock));
}
