use crate::session_state::TypedSession;
use crate::utils::{build_error_response, e500};
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::InternalError;
use actix_web::http::StatusCode;
use actix_web::middleware::Next;
use actix_web::{FromRequest, HttpMessage};
use std::ops::Deref;
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn reject_anonymous_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    match session.get_user_id().map_err(e500)? {
        Some(user_id) => {
            req.extensions_mut().insert(UserId(user_id));
            next.call(req).await
        }
        None => {
            let msg = "User has not logged in";
            let response = build_error_response(StatusCode::UNAUTHORIZED, msg.to_string());
            Err(InternalError::from_response(anyhow::anyhow!(msg), response).into())
        }
    }
}

pub async fn reject_non_admin_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    let user_id = session.get_user_id().map_err(e500)?.ok_or_else(|| {
        let msg = "User has not logged in";
        let response = build_error_response(StatusCode::UNAUTHORIZED, msg.to_string());
        InternalError::from_response(anyhow::anyhow!(msg), response)
    })?;

    // we will only ever hit this if the session is somehow corrupted
    let is_admin = session.get_is_admin().map_err(e500)?.ok_or_else(|| {
        let msg = "Missing admin flag in session";
        let response = build_error_response(StatusCode::UNAUTHORIZED, msg.to_string());
        InternalError::from_response(anyhow::anyhow!(msg), response)
    })?;

    if !is_admin {
        let msg = "Admin privileges required";
        let response = build_error_response(StatusCode::FORBIDDEN, msg.to_string());
        return Err(InternalError::from_response(anyhow::anyhow!(msg), response).into());
    }

    req.extensions_mut().insert(user_id);
    next.call(req).await
}
