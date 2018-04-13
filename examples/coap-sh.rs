extern crate tokio_coap;
extern crate tokio;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::net::SocketAddr;

use tokio::prelude::{Future, Stream, Sink};
use tokio::net::{UdpFramed, UdpSocket};

use tokio_coap::codec::CoapCodec;
use tokio_coap::message::{Mtype, Code};
use tokio_coap::message::Code::{Content, NotImplemented};
use tokio_coap::message::option::UriPath;

fn main() {
    pretty_env_logger::init();

    let addr: SocketAddr = "0.0.0.0:5683".parse().unwrap();

    let sock = UdpSocket::bind(&addr).unwrap();

    let (sink, stream) = UdpFramed::new(sock, CoapCodec).split();

    let stream = stream.filter_map(|(mut request, addr)| {
        info!("--> {:?}", request);

        match request.mtype {
            Mtype::Confirmable | Mtype::NonConfirmable => {
                let path = request.options.get::<UriPath>();
                match (&request.code, &path) {
                    (&Code::Get, &Some(ref p)) if p == &["ip".into()] => {
                         Some((request.new_reply()
                            .with_code(Content)
                            .with_payload(addr.ip()
                                              .to_string()
                                              .as_bytes()
                                              .to_owned()),
                            addr))
                    }
                    _ => {
                        Some((request.new_reply().with_code(NotImplemented), addr))
                    }
                }
            }
            _ => {
                warn!("<-X Not replying to message of type: {:?}", request.mtype);
                None
            }
        }
    });

    let server = sink.send_all(stream);
    tokio::run(server.map(|_| ()).map_err(|e| error!("error = {:?}", e)));
}
