use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials, UserId},
    routes::admin::dashboard::get_username,
    utils::{err500, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    confirm_new_password: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    if form.new_password.expose_secret() != form.confirm_new_password.expose_secret() {
        FlashMessage::error("Passwords do not match").send();
        return Ok(see_other("/admin/password"));
    }
    if form.new_password.expose_secret().len() < 12 || form.new_password.expose_secret().len() > 128
    {
        FlashMessage::error("New password must be greater than 12 and less than 128 characters")
            .send();
        return Ok(see_other("/admin/password"));
    }
    let username = get_username(*user_id, &connection_pool)
        .await
        .map_err(err500)?;
    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };
    if let Err(e) = validate_credentials(credentials, &connection_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("Current password is incorrect").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(err500(e)),
        };
    }
    crate::authentication::change_password(*user_id, form.0.new_password, &connection_pool)
        .await
        .map_err(err500)?;
    FlashMessage::info("Password successfully updated").send();
    Ok(see_other("/admin/password"))
}
