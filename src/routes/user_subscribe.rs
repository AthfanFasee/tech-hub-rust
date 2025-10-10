use crate::authentication::UserId;
use crate::domain::UserEmail;
use crate::email_client::{EmailClient, EmailError};
use crate::routes::get_user_id_from_token;
use crate::startup::ApplicationBaseUrl;
use crate::utils::generate_token;
use crate::{build_error_response, error_chain_fmt};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct SubscribeParameters {
    token: String,
}

#[derive(thiserror::Error)]
pub enum UserSubscribeError {
    #[error("{0}")]
    ValidationError(String),

    #[error("Invalid subscription token.")]
    UnknownToken,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for UserSubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for UserSubscribeError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            UserSubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            UserSubscribeError::UnknownToken => StatusCode::UNAUTHORIZED,
            UserSubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        build_error_response(status_code, self.to_string())
    }
}

#[tracing::instrument(name = "Confirm a pending user subscription", skip(parameters, pool))]
pub async fn subscribe_user(
    parameters: web::Query<SubscribeParameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, UserSubscribeError> {
    let user_id = get_user_id_from_token(&pool, &parameters.token)
        .await
        .context("Failed to retrieve the user id associated with the provided token.")?
        // Domain error (invalid token), so a new `UserConfirmError::UnknownToken` error is created instead of wrapping an `anyhow::Error`
        .ok_or(UserSubscribeError::UnknownToken)?;

    subscribe_user_and_delete_token(&pool, user_id, &parameters.token)
        .await
        .context("Failed to update the user status as subscribed")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Mark user as subscribed and delete token",
    skip(user_id, pool, token)
)]
pub async fn subscribe_user_and_delete_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        WITH subscribe_user AS (
            UPDATE users
            SET is_subscribed = true
            WHERE id = $1 and is_activated = true
        )
        DELETE FROM tokens
        WHERE token = $2 AND user_id = $1 AND is_subscription = true
        "#,
        user_id,
        token,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Confirm a pending user subscription",
    skip(email_client, pool, base_url)
)]
pub async fn send_subscribe_email(
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, UserSubscribeError> {
    let user_id = user_id.into_inner();
    let user_email = get_user_email(*user_id, &pool).await?;
    let email = UserEmail::parse(user_email).map_err(UserSubscribeError::ValidationError)?;

    let activation_token = generate_token();

    store_subscription_token(&pool, *user_id, &activation_token)
        .await
        .context("Failed to store the user subscription token")?;

    send_subscription_email(&email_client, email, &base_url.0, &activation_token)
        .await
        .context("Failed to send a user activation email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Send a subscription email", skip_all)]
pub async fn send_subscription_email(
    email_client: &EmailClient,
    user_email: UserEmail,
    base_url: &str,
    token: &str,
) -> Result<(), EmailError> {
    let confirmation_link = format!("{base_url}/user/confirm/subscribe?token={token}");
    let plain_body = format!(
        "Welcome to TechHub Newsletter!\nVisit {confirmation_link} to confirm your subscription to our newsletter.",
    );
    let html_body = format!(
        "Welcome to TechHub Newsletter!<br />\
        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription to our newsletter.",
    );
    email_client
        .send_email(&user_email, "Welcome!", &html_body, &plain_body)
        .await
}

#[tracing::instrument(name = "Get user email", skip(pool))]
pub async fn get_user_email(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT email
        FROM users
        WHERE id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a user email.")?;
    Ok(row.email)
}

#[tracing::instrument(name = "Store subscription token in the database", skip(token, pool))]
pub async fn store_subscription_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO tokens (token, user_id, is_subscription)
            VALUES ($1, $2, $3)"#,
        token,
        user_id,
        true,
    )
    .execute(pool)
    .await?;

    Ok(())
}
