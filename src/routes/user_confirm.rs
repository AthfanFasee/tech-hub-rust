use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    token: String,
}

#[derive(thiserror::Error)]
pub enum UserConfirmError {
    #[error("There is no user associated with the provided token.")]
    UnknownToken,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for UserConfirmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::routes::user::error_chain_fmt(self, f)
    }
}

impl ResponseError for UserConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            UserConfirmError::UnknownToken => StatusCode::UNAUTHORIZED,
            UserConfirmError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
#[tracing::instrument(name = "Confirm a pending user activation", skip(parameters, pool))]

pub async fn user_confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, UserConfirmError> {
    let user_id = get_user_id_from_token(&pool, &parameters.token)
        .await
        .context("Failed to retrieve the user id associated with the provided token.")?
        .ok_or(UserConfirmError::UnknownToken)?;

    activate_user(&pool, user_id)
        .await
        .context("Failed to update the user status as activated")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Mark user as activated", skip(user_id, pool))]
pub async fn activate_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE users SET is_activated = true WHERE id = $1"#,
        user_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}
#[tracing::instrument(name = "Get user_id from token", skip(token, pool))]
pub async fn get_user_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT user_id FROM tokens \
            WHERE token = $1",
        token,
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.user_id))
}
