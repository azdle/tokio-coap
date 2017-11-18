extern crate tokio_coap;
extern crate futures;
extern crate tokio_core;
extern crate env_logger;

use std::net::SocketAddr;

use futures::{future, Future, Stream, Sink};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use tokio_coap::message::{Message, Mtype, Code};
use tokio_coap::message::option::{Option, Options, UriPath};

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let local_addr: SocketAddr = "0.0.0.0:5683".parse().unwrap();
    let remote_addr: SocketAddr = "134.102.218.18:5683".parse().unwrap(); // coap.me

    let sock = UdpSocket::bind(&local_addr, &handle).unwrap();

    let framed_socket = sock.framed(tokio_coap::codec::CoapCodec);

    let mut opts = Options::new();
    opts.push(UriPath::new("test".to_owned()).into());

    let request = Message {
        version: 1,
        mtype: Mtype::Confirmable,
        code: Code::Get,
        mid: 5234,
        token: vec![3,36,254,64,0].into(),
        options: opts,
        payload: vec![]
    };

    let client =  framed_socket
        .send((remote_addr, Some(request)))
        .and_then(|x| {
            x
            .take(1) // we expect 1 response packet, TODO: check that packet is response
            .for_each(|(_addr, msg)| {
                match msg {
                    Some(msg) => {
                        match msg.code {
                            Code::Content => {
                                println!("{}", String::from_utf8_lossy(&msg.payload));
                            },
                            _ => println!("Unexpeted Response"),
                        }
                    },
                    None => println!("Got un-parsable packet"),
                }

                future::ok(())
            })
        });

    core.run(client).expect("run core");
}
