extern crate tokio_coap;
extern crate futures;
extern crate tokio_core;
extern crate env_logger;

use std::net::SocketAddr;

use futures::{future, Future, Stream, Sink};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use tokio_coap::message::{Message, Mtype, Code};
use tokio_coap::message::option::Options;

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let local_addr: SocketAddr = "0.0.0.0:5683".parse().unwrap();
    let remote_addr: SocketAddr = "134.102.218.18:5683".parse().unwrap(); // coap.me

    let sock = UdpSocket::bind(&local_addr, &handle).unwrap();

    let framed_socket = sock.framed(tokio_coap::codec::CoapCodec);

    let request = Message {
        version: 1,
        mtype: Mtype::Confirmable,
        code: Code::Get,
        mid: 5234,
        token: vec![3,36,254,64,0],
        options: Options::new(),
        payload: vec![]
    };

    let client =  framed_socket
        .send((remote_addr, Some(request)))
        .and_then(|x| {
            x
            .take(1)
            .for_each(|(addr, msg)| {
                println!("Response from {}: {:?}", addr, msg);

                future::ok(())
            })
        });

    drop(core.run(client));
}
