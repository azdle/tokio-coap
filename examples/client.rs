extern crate tokio_coap;
extern crate tokio;
extern crate futures;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use futures::{Future, Stream, Sink};
use futures::future::ok;

use tokio::net::{UdpFramed, UdpSocket};
use tokio::util::FutureExt;

use tokio_coap::codec::CoapCodec;
use tokio_coap::error::Error;
use tokio_coap::message::{Message, Mtype, Code};
use tokio_coap::message::option::{Option, Options, UriPath};

fn main() {
    pretty_env_logger::init();

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

    info!("sending request");
    let client =  framed_socket
        .send((request, remote_addr))
        .and_then(|sock| {
            let timeout_time = Instant::now() + Duration::from_millis(1000);
            sock
                .take_while(|&(ref msg, ref _addr)| {
                    match msg.code {
                        Code::Content => {
                            info!("{}", String::from_utf8_lossy(&msg.payload));
                            ok(false) // done
                        },
                        _ => {
                            warn!("Unexpeted Response");
                            ok(true) // keep listening for packets
                        },
                    }
                })
                .for_each(|(_msg, _addr)| ok(()))
                .deadline(timeout_time)
                .map_err(|_| Error::Timeout)
        })
        .map_err(|err| {
            error!("error = {:?}", err);
        });


    tokio::run(client);

    info!("[exit]");
}
