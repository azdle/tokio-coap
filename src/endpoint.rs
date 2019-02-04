use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;

use futures::prelude::*;
use futures::future;
use futures::sync::{mpsc, oneshot};

use tokio_dns;

use error::Error;
use client::{/*Client,*/ IoFuture};
use message::{Message, Mtype};
use socket::CoapSocket;

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
    Single(oneshot::Sender<(Message, SocketAddr)>),
    Multiple(mpsc::UnboundedSender<(Message, SocketAddr)>),
}

/// A convenient interface for easily making one or more client style requests.
pub struct Client {
    socket: CoapSocket,
    request_sender: mpsc::UnboundedSender<((Message, SocketAddr), Responder)>,
}

impl Client {
    pub fn request(&self, msg: Message, addr: SocketAddr) -> oneshot::Receiver<(Message, SocketAddr)> {
        let (sender, receiver) = oneshot::channel();
        let responder = Responder::Single(sender);

        self.request_sender.clone().unbounded_send(((msg, addr), responder));

        receiver
    }
}

pub struct Response {
    response_receiver: mpsc::UnboundedReceiver<Message>,
}

impl Stream for Response {
    type Item = Message;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let result = self.response_receiver.poll().map_err(|()| unreachable!());
        trace!("Response::poll -> {:?}", result);
        result
    }
}

#[derive(Debug)]
pub struct Request {
    msg: Message,
    retry_count: u8,
    retry_timeout: (), // not sure if this should live here
    response_sender: mpsc::UnboundedSender<Message>,
}

/// Internal Connection State
#[derive(Debug)]
enum State {
    Idle,
    PollRequestReceiver,
    SendRequest(Message),
    PollResponseReceiver,
    Flush
}

/// While UDP is a connectionless protocol, this library makes use of so-called "connections",
/// which in logically just a pair of endpoints (local, remote), where the local endpoint is taken
/// care of by the Socket and then filters packets (in userspace) to individual connections. This
/// provides a convenient interface for dealing with individual remote devices.
#[derive(Debug)]
pub struct Connection {
    state: State,
    receiver: mpsc::UnboundedReceiver<(Message, SocketAddr)>,
    sender: mpsc::UnboundedSender<(Message, SocketAddr)>,
    remote: SocketAddr,
    next_mid: u16, // currently assumes this doesn't wrap for at least EXCHANGE_LIFETIME
    requests: HashMap<u16, Request>,
    handle_req_receiver: mpsc::UnboundedReceiver<(Message, mpsc::UnboundedSender<Message>)>,
    handle_req_sender: mpsc::UnboundedSender<(Message, mpsc::UnboundedSender<Message>)>,
    socket_return_receiver: mpsc::UnboundedReceiver<Message>,
    socket_return_sender: mpsc::UnboundedSender<Message>,
    socket_req_sender: mpsc::UnboundedSender<(Message, SocketAddr)>,
}

impl Connection {
    pub fn new(remote: SocketAddr, socket_req_sender: mpsc::UnboundedSender<(Message, SocketAddr)>) -> Connection {
        let (sender, receiver) = mpsc::unbounded();
        let (handle_req_sender, handle_req_receiver) = mpsc::unbounded();
        let (socket_return_sender, socket_return_receiver) = mpsc::unbounded();
        Connection {
            state: State::Idle,
            receiver,
            // HACK
            sender,
            remote,
            next_mid: 0, //TODO: Randomize
            requests: HashMap::new(),
			handle_req_receiver,
			handle_req_sender,
			socket_return_receiver,
			socket_return_sender,
            socket_req_sender,
        }
    }

    pub fn handle_msg(&self, msg: Message) {
        println!("From {:?} message: {:?}", self.remote, msg);

        match msg.mtype {
            Mtype::Confirmable => (),
            Mtype::NonConfirmable => (),
            Mtype::Acknowledgment => (),
            Mtype::Reset => (),
        }

        match self.requests.get(&msg.mid) {
            Some(request) => {
                request.response_sender.unbounded_send(msg).unwrap();
            },
            None => {
                warn!("received message with now matching msg id: {}", msg.mid);
            }
        }
    }

    pub fn send(mut self, msg: Message) -> Response {
        debug!("sending on connection: {:?}", msg);
        
        self.socket_req_sender.clone().send((msg.clone(),self.remote));

        let (response_sender, response_receiver) = mpsc::unbounded();
        self.record_request(msg, response_sender);

        Response {
            response_receiver
        }
    }

    fn record_request(&mut self, msg: Message, response_sender: mpsc::UnboundedSender<Message>) {
        trace!("record request");
        let request = Request {
            msg: msg,
            retry_count: 0,
            retry_timeout: (), // not sure if this should live here
            response_sender,
        };

        self.requests.insert(request.msg.mid, request);

        debug!("list of outstanding requests: {:?}", self.requests);
    }

    pub fn remote_addr(&self) -> &SocketAddr {
        &self.remote
    }

    pub fn handle(&self) -> ConnectionHandle {
        ConnectionHandle{
            sender: self.handle_req_sender.clone(),
            socket_return_sender: self.socket_return_sender.clone(),
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        debug!("dropping connection");
    }
}

impl Future for Connection {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use std::mem;

        let mut idle_count = 0;

        loop {
            let state = mem::replace(&mut self.state, State::Idle);
            let (new_state, cont) = match state {
                State::Idle => {
                    (State::PollRequestReceiver, true)
                },
                State::PollRequestReceiver => {
                    match self.handle_req_receiver.poll() {
                        Ok(Async::Ready(Some(req))) => {
                            trace!("handle req receiver channel was ready");
                            debug!("Got request: {:?}", req);

                            idle_count = 0;

                            let (msg, response_chan) = req;
                            self.record_request(msg.clone(), response_chan);
                            (State::SendRequest(msg), true)
                        },
                        Ok(Async::Ready(None)) => {
                            warn!("handle req receiver channel stream has ended");
                            panic!("UdpFramed Stream ended");
                        },
                        Ok(Async::NotReady) => {
                            trace!("handle req receiver channel not ready");
                            idle_count += 1;
                            (State::PollResponseReceiver, true)
                        },
                        Err(e) => {
                            error!("req channel produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::PollResponseReceiver => {
                    match self.socket_return_receiver.poll() {
                        Ok(Async::Ready(Some(msg))) => {
                            trace!("socket return receiver channel was ready");
                            debug!("Got response: {:?}", msg);

                            idle_count = 0;

                            self.handle_msg(msg);
                            (State::Idle, true)
                        },
                        Ok(Async::Ready(None)) => {
                            warn!("socket return receiver channel stream has ended");
                            panic!("UdpFramed Stream ended");
                        },
                        Ok(Async::NotReady) => {
                            trace!("socket return receiver channel not ready");
                            idle_count += 1;
                            (State::Idle, false)
                        },
                        Err(e) => {
                            error!("socket return produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::SendRequest(msg) => {
                    let req = (msg, self.remote);
                    match self.socket_req_sender.start_send(req) {
                        Ok(AsyncSink::Ready) => {
                            trace!("req sent");
                            (State::Flush, true)
                        },
                        Ok(AsyncSink::NotReady(req)) => {
                            trace!("socket was not ready to send");
                            let (msg, _addr) = req;

                            (State::SendRequest(msg), false)
                        },
                        Err(e) => {
                            error!("sending on socekt produced error: {:?}", e);
                            // TODO: Handle Error Somehow
                            panic!("unhandled error in error-less future");
                        }
                    }
                },
                State::Flush => {
                    match self.socket_req_sender.poll_complete() {
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

            if !cont || idle_count == 2 {
                return Ok(Async::NotReady);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    /// For the Handle user to push outgoing messages to the `Connection`
    sender:  mpsc::UnboundedSender<(Message, mpsc::UnboundedSender<Message>)>,
    /// For the `CoapSocket` to push incoming to the `Connection`
    socket_return_sender: mpsc::UnboundedSender<Message>,
}


impl ConnectionHandle {
    pub fn send(mut self, msg: Message) -> Response {
        debug!("sending on connection handle: {:?}", msg);
        let (response_sender, response_receiver) = mpsc::unbounded();

        self.sender.clone().unbounded_send((msg, response_sender)).unwrap();

        Response {
            response_receiver,
        }
    }

    pub fn handle_msg(&self, msg: Message) {
        debug!("sending message to connection");
        self.socket_return_sender.clone().unbounded_send(msg);
    }
}
