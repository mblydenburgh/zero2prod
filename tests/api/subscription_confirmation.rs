use reqwest::Url;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

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
    app.post_subscription(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        String::from(links[0].as_str())
    };
    let raw_confirm_link = &get_link(&body["HtmlBody"].as_str().unwrap());
    let mut confirm_link = Url::parse(raw_confirm_link).unwrap();
    assert_eq!(confirm_link.host_str().unwrap(), "127.0.0.1");
    confirm_link.set_port(Some(app.port)).unwrap();

    let response = reqwest::get(confirm_link).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}
