extern crate tokio_coap;
extern crate tokio;
extern crate futures;
extern crate env_logger;

use std::net::SocketAddr;

use futures::{future, Future, Stream, Sink};
use tokio::net::{UdpFramed, UdpSocket};

use tokio_coap::codec::CoapCodec;
use tokio_coap::message::{Message, Mtype, Code};
use tokio_coap::message::option::{Option, Options, UriPath};

fn main() {
    drop(env_logger::init());

    let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let remote_addr: SocketAddr = "104.236.199.143:5683".parse().unwrap(); // coap.sh

    let sock = UdpSocket::bind(&local_addr).unwrap();

    let framed_socket = UdpFramed::new(sock, CoapCodec);

    let mut opts = Options::new();
    opts.push(UriPath::new("ip".to_owned()));

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
        .send((request, remote_addr))
        .and_then(|x| {
            x
            .take(1) // we expect 1 response packet, TODO: check that packet is response
            .for_each(|(msg, _addr)| {
                match msg.code {
                    Code::Content => {
                        println!("{}", String::from_utf8_lossy(&msg.payload));
                    },
                    _ => println!("Unexpeted Response"),
                };

                future::ok(())
            })
        })
        .map_err(|err| {
            println!("error = {:?}", err);
        });


    tokio::run(client);

    println!("[exit]");
}
