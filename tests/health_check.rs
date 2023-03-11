use sqlx::{PgConnection, Connection};
use zero2prod::configuration::get_configuration;
use std::net::TcpListener;

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::startup::run(listener).expect("Failed to bind server address");
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();
    let client = reqwest::Client::new();
    let response = client.get(&format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to reach health check endpoint");
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subsribe_returns_200_for_valid_req() {
    let address = spawn_app();
    let configuation = get_configuration().expect("Could not read config file");
    let connection_string = configuation.database.connection_string();
    let mut db_connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to db");
    let client = reqwest::Client::new();
    let body = "name=bob%20bobbington&email=bob%40test.com";
    let response = client
        .post(format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
    .fetch_one(&mut db_connection)
    .await
    .expect("Failed to fetch saved subscription");
    assert_eq!(saved.name, "bob bobbington");
    assert_eq!(saved.email, "bob@test.com");
}

#[tokio::test]
async fn subsribe_returns_400_for_invalid_req() {
    let address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=bob%20bobbington", "missing email"),
        ("email=test%40test.com", "missing name"),
        ("", "missing both name and email")
    ];
    for (body, error_message) in test_cases {
    let response = client
        .post(format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(400, response.status().as_u16(), "The API did not fail with 400 Bad Request when the payload was {}", error_message);

    }
    }
