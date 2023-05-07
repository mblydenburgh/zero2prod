use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::{
    session_state::TypedSession,
    utils::{err500, see_other},
};

pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(err500)?.is_none() {
        Ok(see_other("/login"))
    } else {
        session.logout();
        FlashMessage::info("Logout successful").send();
        Ok(see_other("/login"))
    }
}
