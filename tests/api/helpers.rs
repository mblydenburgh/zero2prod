use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
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

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp {
    pub address: String,
    pub connection_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
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
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_confirm_link = String::from(links[0].as_str());
            let mut confirm_link = Url::parse(&raw_confirm_link).unwrap();
            assert_eq!(confirm_link.host_str().unwrap(), "127.0.0.1");
            confirm_link.set_port(Some(self.port)).unwrap();
            confirm_link
        };
        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }
    pub async fn post_newsletter(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }
    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user");
    }
}

pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked, the code in `TRACING` is executed. All other times
    // invocations will skip execution.
    Lazy::force(&TRACING);
    let email_server = MockServer::start().await;
    let config = {
        let mut c = get_configuration().expect("Failed to get configuration");
        c.database.name = uuid::Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };
    configure_database(&config.database).await;
    let application = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = application.port();
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());
    let app = TestApp {
        address,
        connection_pool: get_connection_pool(&config.database),
        email_server,
        port,
        test_user: TestUser::generate(),
    };
    app.test_user.store(&app.connection_pool).await;
    app
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
