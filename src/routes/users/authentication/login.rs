use crate::authentication::AuthError;
use crate::authentication::{Credentials, validate_credentials};
use crate::session_state::TypedSession;
use crate::{build_error_response, error_chain_fmt};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::Context;
use secrecy::Secret;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
        };

        build_error_response(status_code, self.to_string())
    }
}

#[derive(serde::Deserialize)]
pub struct LoginData {
    user_name: String,
    password: Secret<String>,
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
    let credentials = Credentials {
        user_name: payload.0.user_name,
        password: payload.0.password,
    };

    tracing::Span::current().record("user_name", tracing::field::display(&credentials.user_name));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    let is_admin = is_admin_user(user_id, &pool).await?;

    // prevent session fixation attacks with `session.renew()`
    session.renew();
    session.insert_user_id(user_id)?;
    session.insert_is_admin(is_admin)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn log_out(session: TypedSession) -> Result<HttpResponse, LoginError> {
    session.log_out();
    Ok(HttpResponse::Ok().finish())
}

pub async fn protected_endpoint() -> Result<HttpResponse, LoginError> {
    Ok(HttpResponse::Ok().finish())
}

pub async fn is_admin_user(user_id: Uuid, pool: &PgPool) -> Result<bool, anyhow::Error> {
    let record = sqlx::query!(
        r#"
        SELECT is_admin
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to fetch admin flag for user")?;

    let is_admin = record
        .map(|r| r.is_admin)
        .ok_or_else(|| anyhow::anyhow!("No user found"))?;

    Ok(is_admin)
}
