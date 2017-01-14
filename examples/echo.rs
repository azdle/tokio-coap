extern crate tokio_coap;
extern crate futures;
extern crate tokio_core;
extern crate env_logger;

use std::net::SocketAddr;

use futures::{Stream, Sink};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use tokio_coap::message::{Message, Mtype, Code};

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let addr: SocketAddr = "0.0.0.0:5683".parse().unwrap();

    let sock = UdpSocket::bind(&addr, &handle).unwrap();

    let (sink, stream) = sock.framed(tokio_coap::codec::CoapCodec).split();

    let stream = stream.map(|(addr, request)| {
        println!("--> {:?}", request);

        if let Some(request) = request {
            match request.mtype {
                Mtype::Confirmable | Mtype::NonConfirmable => {
                    let reply = Message {
                        version: 1,
                        mtype: Mtype::Acknowledgement,
                        code: Code::NotImplemented,
                        mid: request.mid,
                        token: request.token.clone(),
                        options: vec![],
                        payload: vec![],
                    };

                    println!("<-- {:?}", reply);

                    (addr, Some(reply))
                }
                _ => {
                    println!("<-X Not replying to message of type: {:?}", request.mtype);
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
