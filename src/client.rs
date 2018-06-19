use codec::CoapCodec;
use Endpoint;
use error::{Error, UrlError};
use message::{Message, Code};
use message::option::{Option, Options, UriPath, UriHost, UriQuery};

use std::borrow::Cow;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use futures::prelude::*;

use tokio::net::{UdpSocket, UdpFramed};
use tokio::util::FutureExt;

use percent_encoding::percent_decode;
use url::Url;

/// An alias for the futures produced by this library.
pub type IoFuture<T> = Box<Future<Item = T, Error = Error> + Send>;

pub struct Client {
    /// the remote endpoint to contact
    endpoint: Endpoint,
    /// the message to be sent
    msg: Message,
}

fn depercent(s: &str) -> Result<String, UrlError> {
    percent_decode(s.as_bytes())
        .decode_utf8()
        .map(Cow::into_owned)
        .map_err(UrlError::NonUtf8)
}

/// RFC 7252: 6.4.  Decomposing URIs into Options
fn decompose(url: &Url) -> Result<(Endpoint, Options), UrlError> {
    use url::Host;

    let mut options = Options::new();

    // Step 3, TODO: Support coaps
    match url.scheme() {
        "coap" => (),
        other => Err(UrlError::UnsupportedScheme(other.to_string()))?,
    }

    // Step 4
    if url.fragment().is_some() {
        Err(UrlError::FragmentSpecified)?;
    }

    // Step 6
    let port = url.port().unwrap_or(5683);

    // Step 5
    let endpoint = match url.host().ok_or(UrlError::NonAbsolutePath)? {
        Host::Domain(domain) => {
            // ! gross hack warning !
            // The URL standard from whatwg (which the url crate follows) specifies that you try to
            // parse an IPv6 address no matter whatm but you only try to parse an IPv4 address from
            // a set of "special" url schemes that it defines. *Shockingly* coap isn't one of them.
            // See https://url.spec.whatwg.org/#host-parsing
            // and https://url.spec.whatwg.org/#url-miscellaneous
            //
            // This forces us to try to parse any domain name as an IPv4 address here to comply
            // with the coap spec.

            if let Ok(ip) = domain.parse::<Ipv4Addr>() {
                Endpoint::Resolved((ip, port).into())
            } else {
                let host = domain.to_lowercase();
                options.push(UriHost::new(host.clone()));
                Endpoint::Unresolved(host, port)
            }
        },
        Host::Ipv4(ip) => Endpoint::Resolved((ip, port).into()),
        Host::Ipv6(ip) => Endpoint::Resolved((ip, port).into()),
    };

    // Step 8
    if url.path() != "" && url.path() != "/" {
        for segment in url.path_segments().ok_or(UrlError::NonAbsolutePath)? {
            options.push(UriPath::new(depercent(segment)?));
        }
    }

    // Step 9
    let query = url.query().unwrap_or("");
    if !query.is_empty() {
        for segment in query.split('&') {
            options.push(UriQuery::new(depercent(segment)?));
        }
    }

    Ok((endpoint, options))
}

impl Client {
    pub fn new() -> Client {
        Client {
            endpoint: Endpoint::Unset,
            msg: Message::new(),
        }
    }

    pub fn get(url: &str) -> Result<Client, Error> {
        let mut client = Client::new();
        let url = Url::parse(url).map_err(UrlError::Parse)?;

        let (endpoint, options) = decompose(&url)?;

        client.set_endpoint(endpoint);
        client.msg.options = options;

        Ok(client)
    }

    pub fn set_endpoint(&mut self, endpoint: Endpoint) {
        self.endpoint = endpoint;
    }

    pub fn with_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.set_endpoint(endpoint);

        self
    }

    pub fn send(self) -> IoFuture<Message> {
        let local_addr = "0.0.0.0:0".parse().unwrap();

        let Self { endpoint, msg } = self;
        let client_request = endpoint
            .resolve()
            .and_then(move |remote_addr| {
                let sock = UdpSocket::bind(&local_addr).unwrap();

                let framed_socket = UdpFramed::new(sock, CoapCodec);

                info!("sending request");
                let client =  framed_socket
                    .send((msg, remote_addr))
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

#[cfg(test)]
mod tests {
    use super::decompose;
    use endpoint::Endpoint;
    use message::option::{Option, Options, UriHost, UriPath, UriQuery};

    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    use url::Url;

    #[test]
    fn uri_decompose_normalization() {
        let uri1 = Url::parse("coap://example.com:5683/~sensors/temp.xml").unwrap();
        let uri2 = Url::parse("coap://EXAMPLE.com/%7Esensors/temp.xml").unwrap();
        let uri3 = Url::parse("coap://EXAMPLE.com:/%7esensors/temp.xml").unwrap();

        assert_eq!(decompose(&uri1).unwrap(), decompose(&uri2).unwrap());
        assert_eq!(decompose(&uri2).unwrap(), decompose(&uri3).unwrap());
    }

    #[test]
    fn uri_decompose_basic_ipv6() {
        let uri = Url::parse("coap://[2001:db8::2:1]/").unwrap();

        let sa_ref = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 2, 1)), 5683);
        let opt_ref = Options::new();

        let (endpoint, options) = decompose(&uri).unwrap();

        assert_eq!(endpoint, Endpoint::Resolved(sa_ref));
        assert_eq!(options, opt_ref);
    }

    #[test]
    fn uri_decompose_basic_example_net() {
        let uri = Url::parse("coap://example.net/").unwrap();

        let opt_ref = {
            let mut opts = Options::new();
            opts.push(UriHost::new("example.net".to_string()));
            opts
        };

        let (endpoint, options) = decompose(&uri).unwrap();

        assert_eq!(endpoint, Endpoint::Unresolved("example.net".to_string(), 5683));
        assert_eq!(options, opt_ref);
    }

    #[test]
    fn uri_decompose_example_net_well_known_core() {
        let uri = Url::parse("coap://example.net/.well-known/core").unwrap();

        let opt_ref = {
            let mut opts = Options::new();
            opts.push(UriHost::new("example.net".to_string()));
            opts.push(UriPath::new(".well-known".to_string()));
            opts.push(UriPath::new("core".to_string()));
            opts
        };

        let (endpoint, options) = decompose(&uri).unwrap();

        assert_eq!(endpoint, Endpoint::Unresolved("example.net".to_string(), 5683));
        assert_eq!(options, opt_ref);
    }

    #[test]
    fn uri_decompose_punny_unicode() {
        let uri = Url::parse(
            "coap://xn--18j4d.example/%E3%81%93%E3%82%93%E3%81%AB%E3%81%A1%E3%81%AF"
        ).unwrap();

        let opt_ref = {
            let mut opts = Options::new();
            opts.push(UriHost::new("xn--18j4d.example".to_string()));
            opts.push(UriPath::new("こんにちは".to_string()));
            opts
        };

        let (endpoint, options) = decompose(&uri).unwrap();

        assert_eq!(endpoint, Endpoint::Unresolved("xn--18j4d.example".to_string(), 5683));
        assert_eq!(options, opt_ref);
    }

    #[test]
    fn uri_decompose_port_evil() {
        let uri = Url::parse("coap://198.51.100.1:61616//%2F//?%2F%2F&?%26").unwrap();

        let sa_ref = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)), 61616);
        let opt_ref = {
            let mut opts = Options::new();
            opts.push(UriPath::new("".to_string()));
            opts.push(UriPath::new("/".to_string()));
            opts.push(UriPath::new("".to_string()));
            opts.push(UriPath::new("".to_string()));
            opts.push(UriQuery::new("//".to_string()));
            opts.push(UriQuery::new("?&".to_string()));
            opts
        };

        let (endpoint, options) = decompose(&uri).unwrap();

        assert_eq!(endpoint, Endpoint::Resolved(sa_ref));
        assert_eq!(options, opt_ref);
    }
}
