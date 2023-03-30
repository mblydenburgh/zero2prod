use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{Application, get_connection_pool},
    telemetry::{get_subscriber, init_subscriber},
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

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked, the code in `TRACING` is executed. All other times
    // invocations will skip execution.
    Lazy::force(&TRACING);
    let config = {
        let mut c = get_configuration().expect("Failed to get configuration");
        c.database.name = uuid::Uuid::new_v4().to_string();
        c.application.port = 0;
        c
    };
    configure_database(&config.database).await;
    let application = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());
    TestApp {
        address,
        connection_pool: get_connection_pool(&config.database),
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
