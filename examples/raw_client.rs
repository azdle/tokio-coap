extern crate pretty_env_logger;
extern crate tokio_coap;
extern crate tokio;
extern crate futures;
extern crate url;
#[macro_use]
extern crate log;

use tokio_coap::endpoint::Endpoint;
use tokio_coap::message::Message;
use tokio_coap::socket::CoapSocket;

use futures::Future;
use futures::Stream;

use tokio::runtime::Runtime;


fn main() {
    if ::std::env::var("RUST_LOG").is_err() {
        ::std::env::set_var("RUST_LOG", "trace");
    }

    pretty_env_logger::init();

    let mut runtime = Runtime::new().expect("failed to create tokio runtime");

    let mut socket = CoapSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
    let handle = socket.handle();

    println!("listening on {}", socket.local_addr().unwrap());

    runtime.spawn(socket);

    let request = handle.connect(Endpoint::Unresolved("127.0.0.1".into(), 5683))
        .and_then(|mut connection| {
            info!("Connected: {:?}", connection);
            let request = Message::new();
            let r = connection.send(request).take(1).for_each(|response| {
                info!("got response: {:?}", response);
                Ok(())
            }).and_then(|_| {
                info!("responses finished");
                Ok(())
            });
            info!("end of connect handler");
            return r;
        });

    runtime.block_on(request).expect("failed to run request on runtime");
}
