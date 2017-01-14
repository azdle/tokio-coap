extern crate futures;
extern crate tokio_core;
extern crate env_logger;

use std::str;
use std::io::{Result, Error, ErrorKind, Write};
use std::net::SocketAddr;

use futures::{Stream, Sink};
use tokio_core::net::{UdpSocket, UdpCodec};
use tokio_core::reactor::Core;

pub struct IntCodec;

fn parse_u64(buf: &[u8]) -> Result<u64> {
    str::from_utf8(buf)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?
        .parse()
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

impl UdpCodec for IntCodec {
    type In = (SocketAddr, u64);
    type Out = (SocketAddr, u64);

    fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> Result<Self::In> {
        Ok((*addr, parse_u64(buf)?))
    }

    fn encode(&mut self, (addr, num): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
        writeln!(into, "{}", num).unwrap();
        addr
    }
}

fn main() {
    drop(env_logger::init());

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let addr: SocketAddr = "0.0.0.0:5682".parse().unwrap();

    let sock = UdpSocket::bind(&addr, &handle).unwrap();

    let (sink, stream) = sock.framed(IntCodec).split();

    let stream = stream.map(|(addr, num)| {
        println!("[b] recv: {}", &num);
        (addr, num * 2)
    });

    let sock = sink.send_all(stream);
    drop(core.run(sock));
}
