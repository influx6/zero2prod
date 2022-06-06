use std::future::{ready, Ready};

use actix_session::{Session, SessionExt};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user_id(&self, user_id: Uuid) -> Result<(), serde_json::Error> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    pub fn get_user_id(&self) -> Result<Option<Uuid>, serde_json::Error> {
        self.0.get(Self::USER_ID_KEY)
    }

    pub fn log_out(self) {
        self.0.purge()
    }
}

// We implement `FromRequest` an actix_web extractor which we can
// implement on a type to allow it to be used as an actix web extractor.
impl FromRequest for TypedSession {
    // We are saying we return the same error as the base trait type `Session`.
    type Error = <Session as FromRequest>::Error;

    // Rust does not yet support `async` syntax in traits, `FromRequest` expects
    // a future as return type to allow for extractors
    // that need to perform async ops.
    // We do not do async op, so we will wrap `TypeSession` into a `Ready` to convert
    // into a future object resolves to it's wrapped value the first time its polled by
    // executor.
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
