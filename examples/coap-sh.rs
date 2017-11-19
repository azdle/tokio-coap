extern crate tokio_coap;
extern crate futures;
extern crate tokio_core;
extern crate env_logger;

use std::net::SocketAddr;

use futures::{Stream, Sink};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use tokio_coap::message::{Message, Mtype, Code};
use tokio_coap::message::option::{Option, Options, OptionType, OptionKind, UriPath};

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
                    match req.options.get_all_of(OptionKind::UriPath) {
                        Some(x) if x == &vec![OptionType::UriPath(UriPath::new("ip".into()))] => {
                            let reply = Message {
                                version: 1,
                                mtype: Mtype::Acknowledgement,
                                code: Code::Content,
                                mid: req.mid,
                                token: req.token.clone(),
                                options: Options::new(),
                                payload: addr.ip().to_string().as_bytes().to_owned(),
                            };

                            println!("<-- {:?}", reply);

                            (addr, Some(reply))
                        },
                        _ => {
                            let reply = Message {
                                version: 1,
                                mtype: Mtype::Acknowledgement,
                                code: Code::NotImplemented,
                                mid: req.mid,
                                token: req.token.clone(),
                                options: Options::new(),
                                payload: vec![],
                            };

                            println!("<-- {:?}", reply);

                            (addr, Some(reply))
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
