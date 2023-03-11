use std::net::TcpListener;

use zero2prod::{configuration::get_configuration, startup};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Could not read configuration file");
    let address = format!("127.0.0.1:{}", configuration.app_port);
    let listener = TcpListener::bind(address)?;
    startup::run(listener)?.await
}
