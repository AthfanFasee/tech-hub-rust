use std::fmt::{self, Debug, Formatter};

use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use sqlx::PgPool;
use tracing::Span;

use crate::{
    authentication,
    authentication::{AuthError, Credentials},
    domain::LoginData,
    repository,
    session_state::TypedSession,
    utils,
};

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}

#[tracing::instrument(
    skip_all,
    fields(user_name=tracing::field::Empty)
)]
pub async fn login(
    payload: web::Json<LoginData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, LoginError> {
    // Validate payload (returns generic auth error on validation failure)
    let credentials: Credentials = payload
        .0
        .try_into()
        .map_err(|_| LoginError::AuthError(anyhow::anyhow!("Invalid credentials")))?;

    Span::current().record("user_name", tracing::field::display(&credentials.user_name));

    let user_id = authentication::validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    let is_admin = repository::is_admin_user(user_id, &pool).await?;

    session.renew();
    session.insert_user_id(user_id)?;
    session.insert_is_admin(is_admin)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn log_out(session: TypedSession) -> Result<HttpResponse, LoginError> {
    session.log_out();
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument()]
pub async fn protected_endpoint() -> Result<HttpResponse, LoginError> {
    Ok(HttpResponse::Ok().finish())
}
