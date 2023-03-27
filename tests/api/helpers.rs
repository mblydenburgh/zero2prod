use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    telemetry::{get_subscriber, init_subscriber}, email_client::EmailClient,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub connection_pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");
    let mut configuration = get_configuration().expect("Failed to get configuration");
    configuration.database.name = uuid::Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;
    let sender_email = configuration.email_client.sender().expect("invalid sender email");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.token,
        timeout
    );
    let server = zero2prod::startup::run(listener, connection_pool.clone(), email_client)
        .expect("Failed to bind server address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        connection_pool,
    }
}

async fn configure_database(db_config: &DatabaseSettings) -> PgPool {
    Lazy::force(&TRACING);
    // Create connection to database in order to generate new randomly named table for testing
    let mut connection = PgConnection::connect_with(&db_config.without_db())
        .await
        .expect("Could not connect to database");
    // Run create table with random name
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_config.name).as_str())
        .await
        .expect("Failed to create test database");
    // Run migrations
    let connection_pool = PgPool::connect_with(db_config.with_db())
        .await
        .expect("Failed to create connection pool");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to execute migrations");
    // With migrations ran on table, can return connection pool for use in tests
    connection_pool
}
