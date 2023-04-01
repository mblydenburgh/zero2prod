use crate::helpers::spawn_app;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn confirmations_without_a_token_are_rejected() {
    let app = spawn_app().await;
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // Setup test and mock server response
    let app = spawn_app().await;
    let body = "name=bob%20bobbington&email=bob%40test.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    // Make initial request to get confirm link
    app.post_subscription(body.into()).await;

    // Get the received request off of the mock email server and the confirm link out of it
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirm_link = app.get_confirmation_links(email_request);

    // Make request to confirmation link
    let response = reqwest::get(confirm_link.html).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_a_confirm_link_confirms_a_subscriber_in_db() {
    let app = spawn_app().await;
    let body = "name=bob%20bobbington&email=bob%40test.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    // Make inital request to get confirm link
    app.post_subscription(body.into()).await;

    // Get the received request off of the mock email server and the confirm link out of it
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirm_links = app.get_confirmation_links(email_request);

    // Make request to confirm link
    reqwest::get(confirm_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.connection_pool)
        .await
        .expect("Failed to fetch subscriber");
    assert_eq!(saved.email, "bob@test.com");
    assert_eq!(saved.name, "bob bobbington");
    assert_eq!(saved.status, "confirmed");
}
