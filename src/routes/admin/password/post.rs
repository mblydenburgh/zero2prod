use actix_web::{HttpResponse, web};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret,Secret};
use sqlx::PgPool;

use crate::{session_state::TypedSession, utils::{err500, see_other}, routes::admin::dashboard::get_username, authentication::{Credentials, validate_credentials, AuthError}};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    confirm_new_password: Secret<String>
}

pub async fn change_password(session: TypedSession,form: web::Form<FormData>, connection_pool: web::Data<PgPool>) -> Result<HttpResponse, actix_web::Error> {
    let user_id = session.get_user_id().map_err(err500)?;
    if user_id.is_none() {
        return Ok(see_other("/login"))
    };
    let user_id = user_id.unwrap();
    if form.new_password.expose_secret() != form.confirm_new_password.expose_secret() {
        FlashMessage::error("Passwords do not match").send();
        return Ok(see_other("/admin/password"));
    }
    if form.new_password.expose_secret().len() < 12 || form.new_password.expose_secret().len() > 128 {
        FlashMessage::error("New password must be greater than 12 and less than 128 characters").send();
        return Ok(see_other("/admin/password"));
    }
    let username = get_username(user_id, &connection_pool).await.map_err(err500)?;
    let credentials = Credentials {
        username,
        password: form.0.current_password
    };
    if let Err(e) = validate_credentials(credentials, &connection_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("Current password is incorrect").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(err500(e).into())
        }
    }
    todo!()
}
