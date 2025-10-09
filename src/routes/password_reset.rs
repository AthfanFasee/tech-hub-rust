use crate::authentication::{AuthError, Credentials, validate_credentials};
use crate::routes::{build_error_response, error_chain_fmt};
use crate::session_state::TypedSession;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::Context;
use secrecy::ExposeSecret;
use secrecy::Secret;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum PasswordResetError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PasswordResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PasswordResetError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            PasswordResetError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PasswordResetError::AuthError(_) => StatusCode::UNAUTHORIZED,
            PasswordResetError::BadRequest(_) => StatusCode::BAD_REQUEST,
        };

        build_error_response(status_code, self.to_string())
    }
}

#[derive(Serialize)]
struct SuccessResponse {
    code: u16,
    message: String,
}

#[derive(serde::Deserialize)]
pub struct PasswordResetData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    payload: web::Json<PasswordResetData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, PasswordResetError> {
    let user_id = session.get_user_id()?;

    if user_id.is_none() {
        return Err(PasswordResetError::AuthError(anyhow::anyhow!(
            "User not logged in"
        )));
    };

    let user_id = user_id.unwrap();

    let username = get_username(user_id, &pool).await?;

    let PasswordResetData {
        current_password,
        new_password,
        new_password_check,
    } = payload.into_inner();

    let credentials = Credentials {
        username,
        password: current_password,
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => Err(PasswordResetError::AuthError(e.into())),
            AuthError::UnexpectedError(_) => Err(PasswordResetError::UnexpectedError(e.into())),
        };
    }

    let new_pw_len = new_password.expose_secret().len();
    if !(12..=128).contains(&new_pw_len) {
        return Err(PasswordResetError::BadRequest(
            "New password must be between 12 and 128 characters".into(),
        ));
    }

    if new_password.expose_secret() != new_password_check.expose_secret() {
        return Err(PasswordResetError::BadRequest(
            "New Passwords do not match".into(),
        ));
    };

    crate::authentication::change_password(user_id, new_password, &pool).await?;

    let success = SuccessResponse {
        code: 200,
        message: "Password changed successfully".to_string(),
    };
    Ok(HttpResponse::Ok().json(success))
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT name
        FROM users
        WHERE id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.name)
}
