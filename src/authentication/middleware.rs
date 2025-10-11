use crate::app_error;
use crate::session_state::TypedSession;
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
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| app_error(StatusCode::UNAUTHORIZED, "User has not logged in"))?;

    req.extensions_mut().insert(UserId(user_id));
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
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| app_error(StatusCode::UNAUTHORIZED, "User has not logged in"))?;

    let is_admin = session
        .get_is_admin()
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| app_error(StatusCode::UNAUTHORIZED, "Missing admin flag in session"))?;

    if !is_admin {
        return Err(app_error(
            StatusCode::FORBIDDEN,
            "Admin privileges required",
        ));
    }

    req.extensions_mut().insert(UserId(user_id));
    next.call(req).await
}
