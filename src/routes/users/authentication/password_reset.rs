use crate::authentication::UserId;
use crate::authentication::{AuthError, Credentials, validate_credentials};
use crate::domain::UserPassword;
use crate::{build_error_response, error_chain_fmt};
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
    #[error("Invalid request: {0}")]
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
}

impl TryFrom<PasswordResetData> for (UserPassword, UserPassword) {
    type Error = String;

    fn try_from(payload: PasswordResetData) -> Result<Self, Self::Error> {
        let current_password =
            UserPassword::parse(payload.current_password.expose_secret().to_string())?;
        let new_password = UserPassword::parse(payload.new_password.expose_secret().to_string())?;
        Ok((current_password, new_password))
    }
}

#[tracing::instrument(
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn change_password(
    payload: web::Json<PasswordResetData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PasswordResetError> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &pool).await?;

    let (current_password, new_password) = payload
        .0
        .try_into()
        .map_err(PasswordResetError::BadRequest)?;

    let credentials = Credentials {
        user_name: username,
        password: current_password.into_secret(),
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => Err(PasswordResetError::AuthError(e.into())),
            AuthError::UnexpectedError(_) => Err(PasswordResetError::UnexpectedError(e.into())),
        };
    }

    crate::authentication::change_password(*user_id, new_password.into_secret(), &pool).await?;

    let success = SuccessResponse {
        code: 200,
        message: "Password changed successfully".to_string(),
    };
    Ok(HttpResponse::Ok().json(success))
}

pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_name
        FROM users
        WHERE id = $1 and is_activated = true
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.user_name)
}
