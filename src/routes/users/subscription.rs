use crate::authentication::UserId;
use crate::domain::UserEmail;
use crate::email_client::{EmailClient, EmailError};
use crate::routes::users::authentication::register;
use crate::startup::ApplicationBaseUrl;
use crate::utils;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::Context;
use sqlx::PgPool;
use std::fmt;
use std::fmt::{Debug, Formatter};
use tracing::{Span, field};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct SubscribeUserParameters {
    token: String,
}

#[derive(thiserror::Error)]
pub enum SubscriptionError {
    #[error("{0}")]
    ValidationError(String),

    #[error("Invalid subscription token.")]
    UnknownToken,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for SubscriptionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscriptionError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            SubscriptionError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscriptionError::UnknownToken => StatusCode::UNAUTHORIZED,
            SubscriptionError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}

#[tracing::instrument(
    skip_all,
    fields(user_id=tracing::field::Empty)
)]
pub async fn subscribe_user(
    parameters: web::Query<SubscribeUserParameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, SubscriptionError> {
    let user_id = register::get_user_id_from_token(&pool, &parameters.token)
        .await?
        // Domain error (invalid token), so a new `UserConfirmError::UnknownToken` error is created instead of wrapping an `anyhow::Error`
        .ok_or(SubscriptionError::UnknownToken)?;
    Span::current().record("user_id", field::display(user_id));

    subscribe_user_and_delete_token(&pool, user_id, &parameters.token).await?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(skip(pool, token))]
pub async fn subscribe_user_and_delete_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), anyhow::Error> {
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
    .await
    .context("Failed to update the user status as subscribed")?;

    Ok(())
}

#[tracing::instrument(
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn request_subscription(
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, SubscriptionError> {
    let user_id = user_id.into_inner();
    let user_email = get_user_email(*user_id, &pool).await?;
    let email = UserEmail::parse(user_email).map_err(SubscriptionError::ValidationError)?;

    let activation_token = utils::generate_token();

    store_subscription_token(&pool, *user_id, &activation_token).await?;

    send_subscription_email(&email_client, email, &base_url.0, &activation_token)
        .await
        .context("Failed to send a user subscription email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    skip_all,
    fields(user_email = %user_email)
)]
pub async fn send_subscription_email(
    email_client: &EmailClient,
    user_email: UserEmail,
    base_url: &str,
    token: &str,
) -> Result<(), EmailError> {
    let confirmation_link = format!("{base_url}/v1/user/subscribe?token={token}");
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

pub async fn get_user_email(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT email
        FROM users
        WHERE id = $1 and is_activated = true
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a user email.")?;
    Ok(row.email)
}

#[tracing::instrument(skip(token, pool))]
pub async fn store_subscription_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"INSERT INTO tokens (token, user_id, is_subscription)
            VALUES ($1, $2, $3)"#,
        token,
        user_id,
        true,
    )
    .execute(pool)
    .await
    .context("Failed to store the user subscription token")?;

    Ok(())
}
