use std::net::TcpListener;

use zero2prod::{configuration, routes, startup};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener = TcpListener::bind("http://127.0.0.1:8007").expect("Could not bind to address");
    startup::run(listener)?.await
}
