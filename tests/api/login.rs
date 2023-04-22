use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response =  app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/login");
    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(flash_cookie.value(), "Authentication failed");
}
