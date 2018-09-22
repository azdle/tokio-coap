use std::net::SocketAddr;

use futures::prelude::*;
use futures::sync::{mpsc, oneshot};

use tokio::net::{UdpFramed, UdpSocket};

use error::Error;
use client::{/*Client,*/ IoFuture};
use codec::CoapCodec;
use message::Message;
use endpoint::{Connection, Endpoint};

enum State {
    Idle,
    Send((Message, SocketAddr)),
    Flush,
}

/// A local endpoint. This handles all traffic passing through the local udp endpoint, allowing
/// zero or more `Client`s and zero or one `Server`s to share a single local endpoint.
pub struct CoapSocket {
    socket: UdpFramed<CoapCodec>,
    state: State,
    connection_req_receivers: Vec<mpsc::UnboundedReceiver<oneshot::Sender<Connection>>>,
    connections: Vec<Connection>, // TODO: Hashmap?
}

impl CoapSocket {
    /// Create a new local endpoint from the given `UdpSocket`.
    pub fn new(socket: UdpSocket) -> CoapSocket {
        let socket = UdpFramed::new(socket, CoapCodec);

        CoapSocket {
            socket,
            state: State::Idle,
            connection_req_receivers: Vec::new(),
            connections: Vec::new(),
        }
    }

    /// Create a new local endpoint bound to the given `SocketAddr`.
    pub fn bind(addr: &SocketAddr) -> Result<CoapSocket, Error> {
        let socket = UdpSocket::bind(addr)?;
        Ok(Self::new(socket))
    }

    pub fn local_addr(&self) -> Result<SocketAddr, Error> {
        self.socket.get_ref().local_addr().map_err(|e| e.into())
    }

    pub fn connect<E: Into<Endpoint>>(&self, remote: E) -> IoFuture<Connection> {
        Box::new(remote.into().resolve().and_then(move |addr| {
            debug!("creating connection");
            Ok(Connection::new(addr))
        }))
    }

    fn sort_msg_to_connection(&self, msg: Message, src: SocketAddr) {
        debug!("Received Message: {:?}", msg);

        for connection in &self.connections {
            if src == *connection.remote_addr() {
                connection.handle_msg(msg);
                return;
            }
        }

        // TODO: If `Server` exists, create new `Connection` to service request from new remote.

        warn!("dropping unexpected message from {}", src);
    }

    pub fn handle(&mut self) -> SocketHandle {
        let (sender, receiver) = mpsc::unbounded();

        self.connection_req_receivers.push(receiver);

        SocketHandle {
            conn_req: sender,
        }
    }
}



/// `Socket` implementes a future that is intended to be spawned as a task which should never then
/// resolve.
impl Future for CoapSocket {
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
                        Ok(Async::Ready(Some((msg, src)))) => {
                            trace!("socket was ready");

                            self.sort_msg_to_connection(msg, src);
                        },
                        Ok(Async::Ready(None)) => {
                            warn!("socket stream has ended");
                            panic!("UdpFramed Stream ended");
                        },
                        Ok(Async::NotReady) => {
                            trace!("socket not ready");
                            // nothing to do, let request_receiver also return NotReady
                        },
                        Err(e) => {
                            error!("socket produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    };

                    /*
                    match self.request_receiver.poll() {
                        Ok(Async::Ready(Some(req))) => {
                            trace!("req channel was ready");

                            // TODO: Send off Requests
                            println!("Got request: {:?}", req);
                            (State::Send(req), true)
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
                    */
                    (State::Idle, false)
                },
                State::Send(req) => {
                    match self.socket.start_send(req) {
                        Ok(AsyncSink::Ready) => {
                            trace!("req sent");
                            (State::Flush, true)
                        },
                        Ok(AsyncSink::NotReady(req)) => {
                            trace!("socket was not ready to send");

                            (State::Send(req), false)
                        },
                        Err(e) => {
                            error!("sending on socekt produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::Flush => {
                    match self.socket.poll_complete() {
                        Ok(Async::Ready(())) => {
                            trace!("req flushed");
                            (State::Idle, true)
                        },
                        Ok(Async::NotReady) => {
                            trace!("socket was not ready to flush");
                            (State::Flush, false)
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

#[derive(Clone)]
pub struct SocketHandle {
    conn_req: mpsc::UnboundedSender<oneshot::Sender<Connection>>,
}

impl SocketHandle {
    pub fn connect<E: Into<Endpoint>>(&self, remote: E) -> IoFuture<Connection> {
        Box::new(remote.into().resolve().and_then(move |addr| {
            debug!("creating connection");
            Ok(Connection::new(addr))
        }))
    }
}
