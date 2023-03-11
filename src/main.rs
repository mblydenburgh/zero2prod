use std::net::TcpListener;

use sqlx::{Connection, PgPool};
use zero2prod::{configuration::get_configuration, startup};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Could not read configuration file");
    let connection_pool = PgPool::connect(
        &configuration.database.connection_string()
    )
        .await
        .expect("Failed to connect to database");
    let address = format!("127.0.0.1:{}", configuration.app_port);
    let listener = TcpListener::bind(address)?;
    startup::run(listener, connection_pool)?.await
}
