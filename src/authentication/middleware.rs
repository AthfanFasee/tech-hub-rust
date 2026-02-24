use crate::session_state::TypedSession;
use crate::utils;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
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

#[derive(Copy, Clone, Debug)]
pub struct IsAdmin(bool);

impl std::fmt::Display for IsAdmin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for IsAdmin {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Middleware that rejects requests from unauthenticated users
pub async fn reject_anonymous_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    let user_id = session
        .get_user_id()
        .map_err(|e| utils::app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| utils::app_error(StatusCode::UNAUTHORIZED, "User has not logged in"))?;

    let is_admin = session
        .get_is_admin()
        .map_err(|e| utils::app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| utils::app_error(StatusCode::UNAUTHORIZED, "User has not logged in"))?;

    req.extensions_mut().insert(UserId(user_id));
    req.extensions_mut().insert(IsAdmin(is_admin));
    next.call(req).await
}

// Middleware that rejects requests from non-admin users
pub async fn reject_non_admin_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    let user_id = session
        .get_user_id()
        .map_err(|e| utils::app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| utils::app_error(StatusCode::UNAUTHORIZED, "User has not logged in"))?;

    let is_admin = session
        .get_is_admin()
        .map_err(|e| utils::app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| utils::app_error(StatusCode::UNAUTHORIZED, "Missing admin flag in session"))?;

    if !is_admin {
        return Err(utils::app_error(
            StatusCode::FORBIDDEN,
            "Admin privileges required",
        ));
    }

    req.extensions_mut().insert(UserId(user_id));
    req.extensions_mut().insert(IsAdmin(is_admin));
    next.call(req).await
}
