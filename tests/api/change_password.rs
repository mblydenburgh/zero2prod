use tracing::info;
use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app};

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
        "current_password": Uuid::new_v4().to_string(),
        "new_password": new_password,
        "confirm_new_password": new_password
    });
    let response = app.post_change_password(&body).await;
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let not_matching_password = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": new_password,
        "confirm_new_password": not_matching_password
    });
    info!("TEST USER: {:?}", &app.test_user.username);
    info!("TEST USER: {:?}", &app.test_user.password);
    // Must be logged in to change password
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    }))
    .await;

    let response = app.post_change_password(&body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // Following redirect to assert error message
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Passwords do not match</i></p>"));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": new_password,
        "confirm_new_password": new_password
    });
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    }))
    .await;

    let response = app.post_change_password(&body).await;
    assert_is_redirect_to(&response, "/admin/password");
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Current password is incorrect</i></p>"));
}

#[tokio::test]
async fn new_password_must_be_of_valid_length() {
    let app = spawn_app().await;
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    }))
    .await;
    let long_password = String::from_utf8(vec![b'X'; 130]).unwrap();
    let test_cases = vec![
        (
            "pw",
            "<p><i>New password must be greater than 12 and less than 128 characters</i></p>",
        ),
        (
            &long_password,
            "<p><i>New password must be greater than 12 and less than 128 characters</i></p>",
        ),
    ];
    for (new_password, error_content) in test_cases {
        let body = serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": new_password,
            "confirm_new_password": new_password
        });
        let response = app.post_change_password(&body).await;
        assert_is_redirect_to(&response, "/admin/password");
        let html_page = app.get_change_password_html().await;
        assert!(html_page.contains(error_content));
    }
}
