use crate::authentication;
use crate::domain::{NewUser, UserData, UserEmail, UserName, UserPassword};
use crate::email_client::EmailClient;
use crate::email_client::EmailError;
use crate::startup::ApplicationBaseUrl;
use crate::telemetry;
use crate::utils;
use actix_web::ResponseError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, web};
use anyhow::Context;
use secrecy::ExposeSecret;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use std::fmt;
use std::fmt::{Debug, Formatter};
use tracing::{Span, field};
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum RegisterError {
    // the 0 is something like `self.0` and will print the String value the ValidationError wraps around
    #[error("{0}")]
    ValidationError(String),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for RegisterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for RegisterError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            RegisterError::ValidationError(_) => StatusCode::BAD_REQUEST,
            RegisterError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        user_name = tracing::field::Empty,
        user_email = tracing::field::Empty
    )
)]
pub async fn register_user(
    payload: web::Json<UserData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, RegisterError> {
    // ValidationError doesn't have a from or source hence we have to map this error to the correct enum variant
    let NewUser {
        user_name: name,
        email,
        password,
    } = payload
        .0
        .try_into()
        .map_err(RegisterError::ValidationError)?;

    Span::current().record("user_name", field::display(&name));
    Span::current().record("user_email", field::display(&email));

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let user_id = insert_user(&name, &email, password, &mut transaction).await?;

    let activation_token = utils::generate_token();

    store_activation_token(&mut transaction, user_id, &activation_token).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new user")?;

    send_activation_email(&email_client, email, &base_url.0, &activation_token)
        .await
        .context("Failed to send a user activation email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    skip_all,
    fields(user_name = %user_name)
)]
pub async fn insert_user(
    user_name: &UserName,
    email: &UserEmail,
    password: UserPassword,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, anyhow::Error> {
    let password_hash = telemetry::spawn_blocking_with_tracing(move || {
        authentication::compute_password_hash(password.into_secret())
    })
    .await?
    .context("Failed to hash password")?;

    let user_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
            INSERT INTO users (id, user_name, email, password_hash)
            VALUES ($1, $2, $3, $4)
           "#,
        user_id,
        user_name.as_ref(),
        email.as_ref(),
        password_hash.expose_secret()
    );

    transaction
        .execute(query)
        .await
        .context("Failed to insert new user")?;
    Ok(user_id)
}

#[tracing::instrument(skip(token, transaction))]
pub async fn store_activation_token(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    token: &str,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO tokens (token, user_id, is_activation)
            VALUES ($1, $2, $3)"#,
        token,
        user_id,
        true,
    );

    transaction
        .execute(query)
        .await
        .context("Failed to store the user activation token")?;
    Ok(())
}

#[tracing::instrument(
    skip_all,
    fields(user_email = %user_email)
)]
pub async fn send_activation_email(
    email_client: &EmailClient,
    user_email: UserEmail,
    base_url: &str,
    token: &str,
) -> Result<(), EmailError> {
    let confirmation_link = format!("{base_url}/v1/user/activate?token={token}");
    let plain_body =
        format!("Welcome to TechHub!\nVisit {confirmation_link} to activate your account.",);
    let html_body = format!(
        "Welcome to TechHub!<br />\
        Click <a href=\"{confirmation_link}\">here</a> to activate your account.",
    );
    email_client
        .send_email(&user_email, "Welcome!", &html_body, &plain_body)
        .await
}

#[derive(serde::Deserialize)]
pub struct ActivationParameters {
    token: String,
}

#[derive(thiserror::Error)]
pub enum UserActivationError {
    #[error("There is no user associated with the provided token.")]
    UnknownToken,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for UserActivationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for UserActivationError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            UserActivationError::UnknownToken => StatusCode::UNAUTHORIZED,
            UserActivationError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}

#[tracing::instrument(
    skip_all,
    fields(user_id=tracing::field::Empty)
)]
pub async fn activate_user(
    parameters: web::Query<ActivationParameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, UserActivationError> {
    let user_id = get_user_id_from_token(&pool, &parameters.token)
        .await?
        // Domain error (invalid token), so a new `UserConfirmError::UnknownToken` error is created as there's no existing error to wrap in an `anyhow::Error`
        .ok_or(UserActivationError::UnknownToken)?;
    Span::current().record("user_id", field::display(user_id));

    activate_user_and_delete_token(&pool, user_id, &parameters.token).await?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(skip(pool, token))]
pub async fn activate_user_and_delete_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        WITH activate_user AS (
            UPDATE users
            SET is_activated = true
            WHERE id = $1
        )
        DELETE FROM tokens
        WHERE token = $2 AND user_id = $1 AND is_activation = true
        "#,
        user_id,
        token,
    )
    .execute(pool)
    .await
    .context("Failed to update the user status as activated")?;

    Ok(())
}

pub async fn get_user_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, anyhow::Error> {
    let result = sqlx::query!(
        "SELECT user_id FROM tokens \
            WHERE token = $1",
        token,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to retrieve the user id associated with the provided token.")?;
    Ok(result.map(|r| r.user_id))
}
