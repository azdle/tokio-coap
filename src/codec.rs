use std::option::Option;

use tokio_io::codec::{Decoder,Encoder};
use bytes::BytesMut;

use error::Error;
use message::Message;

pub struct CoapCodec;

impl Encoder for CoapCodec {
    type Item = Message;
    type Error = Error;

    fn encode(&mut self, msg: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        msg.encode_to(dst).map_err(|e| e.into())
    }
}

impl Decoder for CoapCodec {
    type Item = Message;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match Message::decode_from(&buf) {
            Ok(msg) => Ok(Some(msg)),
            Err(_) => Ok(None),
        }
    }
}
