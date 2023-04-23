use std::future::{ready, Ready};

use actix_session::{Session, SessionInsertError, SessionGetError, SessionExt};
use actix_web::FromRequest;
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user(&self, user_id: Uuid) -> Result<(), SessionInsertError> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    pub fn get_user_id(&self) -> Result<Option<Uuid>, SessionGetError> {
        self.0.get(Self::USER_ID_KEY)
    }
}

impl FromRequest for TypedSession {
    // This basically means "return the same error as returned by the implementation of
    // 'FromRequest' for 'Session'
    type Error = <Session as FromRequest>::Error;

    // Rust doesn't support 'async' in traits. 'FromRequest' expects a 'Future' as the return type
    // to allow for extractors that need to perform async operations (like HTTP calls).
    // We dont have a Future here so we wrap 'TypedSession' into 'Ready' to convert it into a
    // Future that resolves to the wrapped value the first time it's polled.
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
