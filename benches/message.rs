#![feature(test)]
extern crate test;
extern crate tokio_coap;

use test::Bencher;
use tokio_coap::message::{Message, Code, Mtype};
use tokio_coap::message::option::Option;

#[bench]
fn test_encode(b: &mut Bencher) {
    b.iter(|| {
        Message {
            version: 1,
            mtype: Mtype::Confirmable,
            code: Code::Empty,
            mid: 0x2354,
            token: vec![34,65],
            options: vec![],
            payload: vec![],
        }.to_bytes().unwrap()
    })
}

#[bench]
fn test_encode_with_opts_with_payload(b: &mut Bencher) {
    b.iter(|| {
        Message {
            version: 1,
            mtype: Mtype::Confirmable,
            code: Code::Post,
            mid: 0x0037,
            token: vec![],
            options: vec![Option::UriPath("1a".to_string()),
                          Option::UriPath("temp".to_string()),
                          Option::UriQuery("a32c85ba9dda45823be416246cf8b433baa068d7"
                              .to_string())],
            payload: vec![0x39, 0x39],
        }.to_bytes().unwrap()
    })
}

#[bench]
fn test_decode(b: &mut Bencher) {
    b.iter(|| {
        let bytes = [0x41, 0x01, 0x00, 0x37, 0x99, 0xFF, 0x01, 0x02];

        Message::from_bytes(&bytes).unwrap()
    })
}

#[bench]
fn test_decode_with_opts_with_payload(b: &mut Bencher) {
    b.iter(|| {
        let bytes = [0x40, 0x02, 0x00, 0x37, 0xb2, 0x31, 0x61, 0x04, 0x74, 0x65, 0x6d, 0x70, 0x4d,
                     0x1b, 0x61, 0x33, 0x32, 0x63, 0x38, 0x35, 0x62, 0x61, 0x39, 0x64, 0x64, 0x61,
                     0x34, 0x35, 0x38, 0x32, 0x33, 0x62, 0x65, 0x34, 0x31, 0x36, 0x32, 0x34, 0x36,
                     0x63, 0x66, 0x38, 0x62, 0x34, 0x33, 0x33, 0x62, 0x61, 0x61, 0x30, 0x36, 0x38,
                     0x64, 0x37, 0xFF, 0x39, 0x39];

        Message::from_bytes(&bytes).unwrap()
    })
}



