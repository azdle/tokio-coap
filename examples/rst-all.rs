extern crate tokio_coap;
extern crate tokio;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::net::SocketAddr;

use tokio::prelude::{Future, Stream, Sink};
use tokio::net::{UdpFramed, UdpSocket};

use tokio_coap::codec::CoapCodec;
use tokio_coap::message::{Message, Mtype, Code};
use tokio_coap::message::option::Options;

fn main() {
    pretty_env_logger::init();

    let addr: SocketAddr = "0.0.0.0:5683".parse().unwrap();

    let sock = UdpSocket::bind(&addr).unwrap();

    let (sink, stream) = UdpFramed::new(sock, CoapCodec).split();

    let stream = stream.filter_map(|(request, addr)| {
        info!("--> {:?}", request);

        match request.mtype {
            Mtype::Confirmable | Mtype::NonConfirmable => {
                let reply = Message {
                    version: 1,
                    mtype: Mtype::Acknowledgement,
                    code: Code::NotImplemented,
                    mid: request.mid,
                    token: request.token.clone(),
                    options: Options::new(),
                    payload: vec![],
                };

                info!("<-- {:?}", reply);

                Some((reply, addr))
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
