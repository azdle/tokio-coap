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

/// While UDP is a connectionless protocol, this library makes use of so-called "connections",
/// which in logically just a pair of endpoints (local, remote), where the local endpoint is taken
/// care of by the Socket and then filters packets (in userspace) to individual connections. This
/// provides a convenient interface for dealing with individual remote devices.
#[derive(Debug)]
pub struct Connection {
    receiver: mpsc::UnboundedReceiver<(Message, SocketAddr)>,
    sender: mpsc::UnboundedSender<(Message, SocketAddr)>,
    remote: SocketAddr,
    next_mid: u16, // currently assumes this doesn't wrap for at least EXCHANGE_LIFETIME
    requests: Vec<Request>,
}

impl Connection {
    pub fn new(remote: SocketAddr) -> Connection {
        let (sender, receiver) = mpsc::unbounded();
        Connection {
            receiver,
            // HACK
            sender,
            remote,
            next_mid: 0, //TODO: Randomize
            requests: Vec::new(),
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

        // HACK
        self.requests[0].response_sender.unbounded_send(msg).unwrap();
    }

    pub fn request(&mut self, msg: Message) -> Response {
        let (response_sender, response_receiver) = mpsc::unbounded();

        let request = Request {
            msg,
            retry_count: 0,
            retry_timeout: (), // not sure if this should live here
            response_sender,
        };

        self.requests.push(request);

        Response {
            response_receiver
        }
    }

    pub fn remote_addr(&self) -> &SocketAddr {
        &self.remote
    }
}

