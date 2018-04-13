use codec::CoapCodec;
use Endpoint;
use error::Error;
use message::{Message, Mtype, Code};
use message::option::{Option, Options, UriPath};

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use futures::prelude::*;

use tokio::net::{UdpSocket, UdpFramed};
use tokio::util::FutureExt;

use tokio_dns;

/// An alias for the futures produced by this library.
pub type IoFuture<T> = Box<Future<Item = T, Error = Error> + Send>;

pub struct Client {
    /// the remote endpoint to contact
    endpoint: Endpoint,
    /// the message to be sent
    msg: Message,
}

impl Client {
    pub fn new() -> Client {
        Client {
            endpoint: Endpoint::Unset,
            msg: Message::new(),
        }
    }

    pub fn get(url: &str) -> Client {
        Client::new()
            .with_endpoint(url.parse().unwrap())
    }

    pub fn set_endpoint(&mut self, endpoint: Endpoint) {
        self.endpoint = endpoint;
    }

    pub fn with_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.set_endpoint(endpoint);

        self
    }

    pub fn send(self) -> IoFuture<Message> {
        //hacks
        let remote_host = "coap.sh";
        let remote_port = 5683;

        let local_addr = "0.0.0.0:0".parse().unwrap();

        let client_request = tokio_dns::resolve::<&str>(remote_host)
            .map_err(|_| Error::Timeout)
            .and_then(move |remote_ip| {
                let remote_addr = SocketAddr::new(remote_ip[0], remote_port);

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
                            .filter_map(|(msg, _addr)| {
                                match msg.code {
                                    Code::Content => {
                                        Some(msg)
                                    },
                                    _ => {
                                        warn!("Unexpeted Response");
                                        None
                                    },
                                }
                            })
                            .take(1)
                            .collect()
                            .map(|mut list| {
                                list.pop().expect("list of one somehow had nothing to pop")
                            })
                            .deadline(timeout_time)
                            .map_err(|_| Error::Timeout)
                    });

                client
            }
        );

        Box::new(client_request)
    }
}



// This doesn't quite work, but leaving it here in case I want to fix & use it
// in the future.
#[allow(unused_macros)]
macro_rules! set_or_with {
    // Opaque Type Options
    ($fn:ident($params:tt) {$body: block}) => {
        pub fn set_$fn($params) {
            $body
        }

        pub fn with_$fn(mut self, $params) -> Self {
            set_$fn(&mut self, $params);

            self
        }
    }
}
