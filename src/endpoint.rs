use std::io;
use std::net::SocketAddr;

use futures::prelude::*;
use futures::future;
use futures::sync::{mpsc, oneshot};

use tokio::net::{UdpFramed, UdpSocket};

use tokio_dns;

use error::Error;
use client::{/*Client,*/ IoFuture};
use codec::CoapCodec;
use message::Message;

#[derive(Debug, PartialEq)]
pub enum Endpoint {
    Unset,
    Resolved(SocketAddr),
    Unresolved(String, u16),
}

impl Endpoint {
    pub fn resolve(self) -> IoFuture<SocketAddr> {
        match self {
            Endpoint::Unset => Box::new(
                future::err(Error::Io(io::Error::new(io::ErrorKind::InvalidInput, "endpoint unset")))
            ),
            Endpoint::Resolved(addr) => Box::new(future::ok(addr)),
            Endpoint::Unresolved(host, port) =>
                Box::new(tokio_dns::resolve::<&str>(&host)
                    .map_err(|e| Error::Io(e))
                    .map(move |ip| SocketAddr::new(ip[0], port))
                ),
        }
    }
}

enum Handlers {
    Client(Client),
//    Server(Server),
}

enum Responder {
    Single(oneshot::Receiver<(Message, SocketAddr)>),
    Multiple(mpsc::UnboundedReceiver<(Message, SocketAddr)>),
}

pub struct Client {
    request_sender: mpsc::UnboundedSender<((Message, SocketAddr), Responder)>,
}

pub struct Request {
    msg: Message,
    retry_count: u8,
    retry_timeout: (), // not sure if this should live here
}

pub struct Connection {
    remote: Endpoint,
    next_mid: u16, // currenly assumes this doesn't wrap for at least EXCHANGE_LIFETIME
    requests: Vec<Request>,
}

enum State {
    Idle,
    Send(((Message, SocketAddr), Responder)),
    Flush(Responder),
}

/// A local endpoint. This handles all traffic passing through the local udp endpoint, allowing
/// zero or more `Client`s and zero or one `Server`s to share a single local endpoint.
pub struct Socket {
    socket: UdpFramed<CoapCodec>,
    state: State,
    connections: Vec<Connection>,
    request_sender: mpsc::UnboundedSender<((Message, SocketAddr), Responder)>,
    request_receiver: mpsc::UnboundedReceiver<((Message, SocketAddr), Responder)>,
}

impl Socket {
    /// Create a new local endpoint from the given `UdpSocket`.
    pub fn new(socket: UdpSocket) -> Socket {
        let socket = UdpFramed::new(socket, CoapCodec);
        let (request_sender, request_receiver) = mpsc::unbounded();

        Socket {
            socket,
            state: State::Idle,
            connections: Vec::new(),
            request_sender,
            request_receiver
        }
    }

    /// Create a new local endpoint bound to the given `SocketAddr`.
    pub fn bind(addr: &SocketAddr) -> Result<Socket, Error> {
        let socket = UdpSocket::bind(addr)?;
        Ok(Self::new(socket))
    }

    pub fn client(&self) -> Client {
        Client {
            request_sender: self.request_sender.clone()
        }
    }
}


/// `Socket` implementes a future that is intended to be spawned as a task which should never then
/// resolve.
impl Future for Socket {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use std::mem;

        loop {
            let state = mem::replace(&mut self.state, State::Idle);
            let (new_state, cont) = match state {
                State::Idle => {
                    // this section can't change our state from idle since we can always push to an
                    // `UnboundedSender` (or we fail immediately)
                    match self.socket.poll() {
                        Ok(Async::Ready(Some((msg, responder)))) => {
                            trace!("socket was ready");

                            // TODO: Sort Responses back to Requester & new requests to server
                            println!("Got msg: {:?}", msg);
                        },
                        Ok(Async::Ready(None)) => {
                            warn!("socket stream has ended");
                            panic!("UdpFramed Stream ended");
                        },
                        Ok(Async::NotReady) => {
                            trace!("socket not ready");
                        },
                        Err(e) => {
                            error!("socket produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    };

                    match self.request_receiver.poll() {
                        Ok(Async::Ready(Some((req, responder)))) => {
                            trace!("req channel was ready");

                            // TODO: Send off Requests & Store Responder
                            println!("Got request: {:?}", req);
                            (State::Send((req, responder)), true)
                        },
                        Ok(Async::Ready(None)) => {
                            warn!("req channel stream has ended");
                            panic!("UdpFramed Stream ended");
                        },
                        Ok(Async::NotReady) => {
                            trace!("req channel not ready");
                            (State::Idle, false)
                        },
                        Err(e) => {
                            error!("req channel produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::Send((req, responder)) => {
                    match self.socket.start_send(req) {
                        Ok(AsyncSink::Ready) => {
                            trace!("req sent");
                            (State::Flush(responder), true)
                        },
                        Ok(AsyncSink::NotReady(req)) => {
                            trace!("socket was not ready to send");

                            (State::Send((req, responder)), false)
                        },
                        Err(e) => {
                            error!("sending on socekt produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::Flush(responder) => {
                    match self.socket.poll_complete() {
                        Ok(Async::Ready(())) => {
                            trace!("req flushed");
                            // TODO: save responder
                            (State::Idle, true)
                        },
                        Ok(Async::NotReady) => {
                            trace!("socket was not ready to flush");
                            (State::Flush(responder), false)
                        },
                        Err(e) => {
                            error!("sending on socket produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
            };

            self.state = new_state;

            if !cont {
                return Ok(Async::NotReady);
            }
        }
    }
}
