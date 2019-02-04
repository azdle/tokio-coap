use std::collections::HashMap;
use std::net::SocketAddr;

use futures::prelude::*;
use futures::sync::{mpsc, oneshot};

use tokio::net::{UdpFramed, UdpSocket};

use error::Error;
use client::{/*Client,*/ IoFuture};
use codec::CoapCodec;
use message::Message;
use endpoint::{Connection, ConnectionHandle, Endpoint};

enum State {
    Idle,
    PollSocket,
    SendSocket((Message, SocketAddr)),
    PollMessageChannel,
    PollConnectionChannel,
    Flush,
}

/// A local endpoint. This handles all traffic passing through the local udp endpoint, allowing
/// zero or more `Client`s and zero or one `Server`s to share a single local endpoint.
pub struct CoapSocket {
    socket: UdpFramed<CoapCodec>,
    state: State,
    connection_req_receiver: mpsc::UnboundedReceiver<(SocketAddr, oneshot::Sender<ConnectionHandle>)>,
    connection_req_sender: mpsc::UnboundedSender<(SocketAddr, oneshot::Sender<ConnectionHandle>)>,
    connections: HashMap<SocketAddr, ConnectionHandle>, // TODO: Hashmap?
    out_msg_sender: mpsc::UnboundedSender<(Message, SocketAddr)>,
    out_msg_receiver: mpsc::UnboundedReceiver<(Message, SocketAddr)>,
}

impl CoapSocket {
    /// Create a new local endpoint from the given `UdpSocket`.
    pub fn new(socket: UdpSocket) -> CoapSocket {
        let socket = UdpFramed::new(socket, CoapCodec);
        let (out_msg_sender, out_msg_receiver) = mpsc::unbounded();
        let (conn_req_sender, conn_req_receiver) = mpsc::unbounded();

        CoapSocket {
            socket,
            state: State::Idle,
            connection_req_receiver: conn_req_receiver,
            connection_req_sender: conn_req_sender,
            connections: HashMap::new(),
            out_msg_sender,
            out_msg_receiver,
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

    pub fn connect<E: Into<Endpoint>>(&self, remote: E) -> IoFuture<ConnectionHandle> {
        self.handle().connect(remote)
    }

    fn get_or_new_connection(&mut self, addr: SocketAddr) -> ConnectionHandle {
        let out_msg_sender = self.out_msg_sender.clone();
        self.connections.entry(addr).or_insert_with(|| {
            info!("new connection for {}", addr);
            let conn = Connection::new(
                addr,
                out_msg_sender,
            );

            let handle = conn.handle();

            tokio::spawn(conn);

            handle
        }).clone()
    }

    fn sort_msg_to_connection(&self, msg: Message, src: SocketAddr) {
        debug!("Received Message: {:?}", msg);

        if let Some(connection) = &self.connections.get(&src) {
            connection.handle_msg(msg);
        } else if false {
            // TODO: hand to server "accept" 
        } else {
            debug!("not handling message from {}, no existing connections and server not setup", src);
            debug!("connections: {:?}", self.connections);
        }
    }

    pub fn handle(&self) -> SocketHandle {
        SocketHandle {
            conn_req: self.connection_req_sender.clone(),
            req_sender: self.out_msg_sender.clone(),
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

        let mut idle_count = 0; 

        // TODO: can I split this so that when sending is blocked I can still work on receiving?
        loop {
            let state = mem::replace(&mut self.state, State::Idle);
            let new_state = match state {
                State::Idle => {
                    // TODO: Round-Robin Other Polling
                    State::PollSocket
                }
                State::PollSocket => {
                    // this section can't change our state from idle since we can always push to an
                    // `UnboundedSender` (or we fail immediately)
                    match self.socket.poll() {
                        Ok(Async::Ready(Some((msg, src)))) => {
                            trace!("socket was ready");
                            idle_count = 0;

                            self.sort_msg_to_connection(msg, src);

                            // the socket needs to be polled again so we get (notified of) the next message
                            // TODO: move to message channel, but prevent infinite loop
                            State::PollSocket
                        },
                        Ok(Async::Ready(None)) => {
                            warn!("socket stream has ended");
                            panic!("UdpFramed Stream ended");
                        },
                        Ok(Async::NotReady) => {
                            trace!("socket not ready");
                            // nothing to do, let request_receiver also return NotReady
                            idle_count += 1;
                            State::PollMessageChannel
                        },
                        Err(e) => {
                            error!("socket produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::PollMessageChannel => {
                    match self.out_msg_receiver.poll() {
                        Ok(Async::Ready(Some(req))) => {
                            trace!("req channel was ready");
                            idle_count = 0;

                            // TODO: Send off Requests
                            println!("Got request: {:?}", req);
                            State::SendSocket(req)
                        },
                        Ok(Async::Ready(None)) => {
                            warn!("req channel stream has ended");
                            panic!("UdpFramed Stream ended");
                        },
                        Ok(Async::NotReady) => {
                            trace!("req channel not ready");
                            idle_count += 1;
                            State::PollConnectionChannel
                        },
                        Err(e) => {
                            error!("req channel produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::PollConnectionChannel => {
                    match self.connection_req_receiver.poll() {
                        Ok(Async::Ready(Some(req))) => {
                            trace!("req channel was ready");
                            idle_count = 0;

                            // TODO: Send off Requests
                            error!("dropping connection request: {:?}", req);
                            let (addr, sender) = req;
                            let conn = self.get_or_new_connection(addr);
                            sender.send(conn);
                            State::PollSocket
                        },
                        Ok(Async::Ready(None)) => {
                            warn!("req channel stream has ended");
                            panic!("UdpFramed Stream ended");
                        },
                        Ok(Async::NotReady) => {
                            trace!("req channel not ready");
                            idle_count += 1;
                            State::PollSocket
                        },
                        Err(e) => {
                            error!("req channel produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::SendSocket(req) => {
                    match self.socket.start_send(req) {
                        Ok(AsyncSink::Ready) => {
                            trace!("req sent");
                            State::Flush
                        },
                        Ok(AsyncSink::NotReady(req)) => {
                            trace!("socket was not ready to send");

                            State::SendSocket(req)
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
                            State::Idle
                        },
                        Ok(Async::NotReady) => {
                            trace!("socket was not ready to flush");
                            State::Flush
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

            if idle_count == 3 {
                return Ok(Async::NotReady);
            }
        }
    }
}

#[derive(Clone)]
pub struct SocketHandle {
    conn_req: mpsc::UnboundedSender<(SocketAddr, oneshot::Sender<ConnectionHandle>)>,
    req_sender: mpsc::UnboundedSender<(Message, SocketAddr)>,
}

impl SocketHandle {
    pub fn connect<E: Into<Endpoint>>(&self, remote: E) -> IoFuture<ConnectionHandle> {
        let req_sender = self.req_sender.clone();
        let conn_req = self.conn_req.clone();
        Box::new(remote.into().resolve().and_then(move |addr| {
            debug!("requesting connection for {}", addr);
            let (conn_sender, conn_receiver) = oneshot::channel();
            conn_req.send((addr, conn_sender))
                .map_err(|e| panic!("con_req send error"))
                .and_then(|_| {
                    conn_receiver.map_err(|e| Error::Canceled(e))
                })
        }))
    }
}
