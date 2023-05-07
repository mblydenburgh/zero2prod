use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_access_admin_dashboard() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    // 1. Login
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // 2. Confirm login page
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", &app.test_user.username)));

    // 3. Post logout
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // 4. Confirm redirect and logout message
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Logout successful</i></p>"#));

    // 5. Attempt to load admin dashboard, confirm redirect back to /login
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login")
}
