use std::io;
use std::net::SocketAddr;
use std::option::Option;

use tokio_core::net::UdpCodec;

use message::Message;

pub struct CoapCodec;

impl UdpCodec for CoapCodec {
    type In = (SocketAddr, Option<Message>);
    type Out = (SocketAddr, Option<Message>);

    fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
        match Message::from_bytes(buf) {
            Ok(msg) => Ok((*addr, Some(msg))),
            Err(_) => Ok((*addr, None)),
        }
    }

    fn encode(&mut self, (addr, mmsg): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
        if let Some(msg) = mmsg {
            let bytes = msg.to_bytes().unwrap();
            into.extend(bytes);
        };

        addr
    }
}