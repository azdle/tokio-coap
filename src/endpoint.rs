use std::io;
use std::net::SocketAddr;

use futures::prelude::*;
use futures::future;

use tokio_dns;

use error::Error;
use client::IoFuture;

pub enum Endpoint {
    Unset,
    Resolved(SocketAddr),
    Unresolved(String, u16),
}

impl Endpoint {
    pub fn resolve(self) -> IoFuture<SocketAddr> {
        match self {
            Endpoint::Unset => Box::new(future::err(Error::Io(io::Error::new(io::ErrorKind::InvalidInput, "endpoint unset")))),
            Endpoint::Resolved(addr) => Box::new(future::ok(addr)),
            Endpoint::Unresolved(host, port) => Box::new(tokio_dns::resolve::<&str>(&host).map_err(|e| Error::Io(e)).map(move |ip| SocketAddr::new(ip[0], port))),
        }
    }
}
