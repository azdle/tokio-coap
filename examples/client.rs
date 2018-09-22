extern crate pretty_env_logger;
extern crate tokio_coap;
extern crate tokio;
extern crate futures;

use tokio_coap::Client;

use futures::Future;
use futures::future::ok;

fn main() {
    if ::std::env::var("RUST_LOG").is_err() {
        ::std::env::set_var("RUST_LOG", "debug");
    }

    pretty_env_logger::init();

    let client = Client::get("coap://coap.sh/ip").unwrap();
    let request = client.send()
        .and_then(|response| {
            println!("response: {}", String::from_utf8_lossy(&response.payload));
            ok(())
        })
        .or_else(|e| {
            println!("error in request: {:?}", e);
            ok(())
        });

    tokio::run(request);
}
