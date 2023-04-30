use uuid::Uuid;

use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let app = spawn_app().await;
    let response = app.get_change_password().await;
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "old_password": Uuid::new_v4().to_string(),
        "new_password": new_password,
        "confirm_new_password": new_password
    });
    let response = app.post_change_password(&body).await;
    assert_is_redirect_to(&response, "/login")
}
