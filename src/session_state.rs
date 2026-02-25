use std::future::{Ready, ready};

use actix_session::{Session, SessionExt};
use actix_web::{FromRequest, HttpRequest, dev::Payload};
use anyhow::Context;
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";
    const IS_ADMIN_KEY: &'static str = "is_admin";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user_id(&self, user_id: Uuid) -> Result<(), anyhow::Error> {
        self.0
            .insert(Self::USER_ID_KEY, user_id)
            .context("Failed to insert user id into the session")
    }

    pub fn insert_is_admin(&self, is_admin: bool) -> Result<(), anyhow::Error> {
        self.0
            .insert(Self::IS_ADMIN_KEY, is_admin)
            .context("Failed to insert admin flag into the session")
    }

    pub fn get_user_id(&self) -> Result<Option<Uuid>, anyhow::Error> {
        self.0
            .get(Self::USER_ID_KEY)
            .context("Failed to get user id from the session")
    }

    pub fn get_is_admin(&self) -> Result<Option<bool>, anyhow::Error> {
        self.0
            .get(Self::IS_ADMIN_KEY)
            .context("Failed to get admin flag from the session")
    }

    pub fn log_out(self) {
        self.0.purge()
    }
}

impl FromRequest for TypedSession {
    type Error = <Session as FromRequest>::Error;
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
