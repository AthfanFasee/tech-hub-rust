use crate::authentication::AuthError;
use crate::authentication::{Credentials, validate_credentials};
use crate::routes::{build_error_response, error_chain_fmt};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use secrecy::Secret;
use sqlx::PgPool;

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
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
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip_all,
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    payload: web::Json<LoginData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, LoginError> {
    let credentials = Credentials {
        username: payload.0.username,
        password: payload.0.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    Ok(HttpResponse::Ok().finish())
}
