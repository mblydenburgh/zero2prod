use crate::authentication::AuthError;
use crate::routes::error_chain_fmt;
use crate::startup::HmacSecret;
use actix_web::HttpResponse;
use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::web;
use actix_web::cookie::Cookie;
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::authentication::{Credentials, validate_credentials};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error)
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
    skip(form, connection_pool, secret), 
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    secret: web::Data<HmacSecret>
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password
    };
    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &connection_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(err) => {
            let err = match err {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into())
            };
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION,"/login"))
                .cookie(Cookie::new("_flash", err.to_string()))
                .finish();
            Err(InternalError::from_response(err, response))
        }
    }
}
