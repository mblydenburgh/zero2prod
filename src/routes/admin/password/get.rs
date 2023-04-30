use actix_web::{HttpResponse, http::header::ContentType};

use crate::{session_state::TypedSession, utils::{err500, see_other}};

pub async fn change_password_form(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(err500)?.is_none() {
        return Ok(see_other("/login"))
    };
    Ok(
        HttpResponse::Ok().content_type(ContentType::html()).body(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content-type="text/html"; charset="utf-8">
    <title>Change Password</title>
</head>
<body>
    <form action="/admin/password" method="post">
        <label>
            Current Password
            <input
                type="password"
                placeholder="Enter current password"
                name="current_password"
            >
        </label>
        <br>
        <label>
            New Password
            <input
                type="password"
                placeholder="Enter new password"
                name="new_password"
            >
        </label>
        <br>
        <label>
            Confirm New Password
            <input
                type="password"
                placeholder="Enter new password again"
                name="confirm_new_password"
            >
        </label>
        <br>
        <button type="submit">Change password</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>
            "#
        )
    )
}
