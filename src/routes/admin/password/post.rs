use actix_web::{HttpResponse, web};
use secrecy::Secret;

use crate::{session_state::TypedSession, utils::{err500, see_other}};

#[derive(serde::Deserialize)]
pub struct FormData {
    old_password: Secret<String>,
    new_password: Secret<String>,
    confirm_new_password: Secret<String>
}

pub async fn change_password(session: TypedSession,form: web::Form<FormData>) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(err500)?.is_none() {
        return Ok(see_other("/login"))
    };

    todo!()
}
