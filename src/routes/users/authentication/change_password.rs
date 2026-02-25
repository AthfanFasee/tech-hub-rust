use std::fmt::{self, Debug, Formatter};

use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use anyhow::Context;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    authentication,
    authentication::{AuthError, Credentials, UserId},
    domain::UserPassword,
    utils,
};

#[derive(thiserror::Error)]
pub enum ChangePasswordError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Invalid request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for ChangePasswordError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for ChangePasswordError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            ChangePasswordError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ChangePasswordError::AuthError(_) => StatusCode::UNAUTHORIZED,
            ChangePasswordError::BadRequest(_) => StatusCode::BAD_REQUEST,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}

#[derive(serde::Deserialize)]
pub struct ChangePasswordData {
    current_password: Secret<String>,
    new_password: Secret<String>,
}

impl TryFrom<ChangePasswordData> for (UserPassword, UserPassword) {
    type Error = String;

    fn try_from(payload: ChangePasswordData) -> Result<Self, Self::Error> {
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
    payload: web::Json<ChangePasswordData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, ChangePasswordError> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &pool).await?;

    let (current_password, new_password) = payload
        .0
        .try_into()
        .map_err(ChangePasswordError::BadRequest)?;

    let credentials = Credentials {
        user_name: username,
        password: current_password.into_secret(),
    };

    if let Err(e) = authentication::validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => Err(ChangePasswordError::AuthError(e.into())),
            AuthError::UnexpectedError(_) => Err(ChangePasswordError::UnexpectedError(e.into())),
        };
    }

    authentication::change_password(*user_id, new_password.into_secret(), &pool).await?;

    Ok(HttpResponse::Ok().finish())
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
