use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subsribe_returns_200_for_valid_req() {
    let app = spawn_app().await;
    let body = "name=bob%20bobbington&email=bob%40test.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_subscription(body.into()).await;
    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.connection_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.name, "bob bobbington");
    assert_eq!(saved.email, "bob@test.com");
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    let body = "name=bob%20bobbington&email=bob%40test.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscription(body.into()).await;
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = app.post_subscription(body.into()).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {description}"
        )
    }
}

#[tokio::test]
async fn subsribe_returns_400_for_invalid_req() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=bob%20bobbington", "missing email"),
        ("email=test%40test.com", "missing name"),
        ("", "missing both name and email"),
    ];
    for (body, error_message) in test_cases {
        let response = app.post_subscription(body.into()).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {error_message}"
        );
    }
}
