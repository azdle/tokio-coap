use std::net::SocketAddr;
use std::str::FromStr;

pub enum Endpoint {
    Unset,
    Resolved(SocketAddr),
    Unresolved(String, u16),
}

impl FromStr for Endpoint {
    type Err = ();
    fn from_str(s: &str) -> Result<Endpoint, ()> {
        Ok(Endpoint::Unresolved(s.to_owned(), 5683))
    }
}
