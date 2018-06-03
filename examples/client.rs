extern crate tokio_coap;
extern crate tokio;
extern crate futures;

use tokio_coap::Client;

use futures::Future;
use futures::future::ok;

fn main() {
    let request = Client::get("coap://coap.sh/ip")
        .unwrap()
        .send()
        .and_then(|response| {
            println!("{}", String::from_utf8_lossy(&response.payload));
            ok(())
        })
        .or_else(|e| {
            println!("error in request: {:?}", e);
            ok(())
        });

    tokio::run(request);
}
