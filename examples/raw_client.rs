extern crate pretty_env_logger;
extern crate tokio_coap;
extern crate tokio;
extern crate futures;
extern crate url;

use tokio_coap::Client;
use tokio_coap::client::decompose;
use tokio_coap::socket::CoapSocket;

use futures::Future;
use futures::future::ok;
use futures::Stream;

use tokio::runtime::Runtime;

use url::Url;

fn main() {
    if ::std::env::var("RUST_LOG").is_err() {
        ::std::env::set_var("RUST_LOG", "trace");
    }

    pretty_env_logger::init();

    let mut runtime = Runtime::new().expect("failed to create tokio runtime");
	let executor = runtime.executor();

    let mut socket = CoapSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
    let handle = socket.handle();

    println!("listening on {}", socket.local_addr().unwrap());

    executor.spawn(socket);

    let mut client = Client::new(handle);

    let url = Url::parse("coap://coap.sh/ip").unwrap();

    let (endpoint, options) = decompose(&url).unwrap();

    client.set_endpoint(endpoint);
    client.msg.options = options;

	let Client { endpoint, msg, socket } = client;

	println!("sending request");
	let request = socket.connect(endpoint)
		.and_then(move |mut connection| {
			connection.request(msg)
				.map(|x| {println!("item: {:?}", x); x})
				.take(1)
				.collect()
				.map(|mut list| {
					println!("list is {:?}", list);
					list.pop().expect("list of one somehow had nothing to pop")
				})
		});

    runtime.block_on(request).unwrap();


    /*
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
    */
}
