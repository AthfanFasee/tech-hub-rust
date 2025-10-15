use crate::authentication::compute_password_hash;
use crate::domain::{NewUser, UserEmail, UserName, UserPassword};
use crate::email_client::EmailClient;
use crate::email_client::EmailError;
use crate::startup::ApplicationBaseUrl;
use crate::telemetry::spawn_blocking_with_tracing;
use crate::utils::generate_token;
use crate::{build_error_response, error_chain_fmt};
use actix_web::ResponseError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, web};
use anyhow::Context;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum UserRegisterError {
    // the 0 is something like `self.0` and will print the String value the ValidationError wraps around
    #[error("{0}")]
    ValidationError(String),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for UserRegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for UserRegisterError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            UserRegisterError::ValidationError(_) => StatusCode::BAD_REQUEST,
            UserRegisterError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        build_error_response(status_code, self.to_string())
    }
}

#[derive(Deserialize)]
pub struct UserData {
    email: String,
    user_name: String,
    password: Secret<String>,
}

// This is like saying - I know how to build myself `NewUser` from something else `UserData`
// Then Rust lets us use `.try_into` whenever there's a `UserData` - where it automatically tries converting it to a `NewUser`
impl TryFrom<UserData> for NewUser {
    type Error = String;

    fn try_from(payload: UserData) -> Result<Self, Self::Error> {
        let user_name = UserName::parse(payload.user_name)?;
        let email = UserEmail::parse(payload.email)?;
        let password = UserPassword::parse(payload.password.expose_secret().to_string())?;
        Ok(Self {
            user_name,
            email,
            password,
        })
    }
}
#[tracing::instrument(
    skip_all,
    fields(
        user_email = %payload.email,
        user_name = %payload.user_name,
    )
)]
pub async fn register_user(
    payload: web::Json<UserData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, UserRegisterError> {
    // ValidationError doesn't have a from or source hence we have to map this error to the correct enum variant
    let NewUser {
        user_name: name,
        email,
        password,
    } = payload
        .0
        .try_into()
        .map_err(UserRegisterError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let user_id = insert_user(&name, &email, password, &mut transaction).await?;

    let activation_token = generate_token();

    store_activation_token(&mut transaction, user_id, &activation_token).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new user")?;

    send_confirmation_email(&email_client, email, &base_url.0, &activation_token)
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
    let password_hash =
        spawn_blocking_with_tracing(move || compute_password_hash(password.into_secret()))
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
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    user_email: UserEmail,
    base_url: &str,
    token: &str,
) -> Result<(), EmailError> {
    let confirmation_link = format!("{base_url}/v1/user/confirm/register?token={token}");
    let plain_body =
        format!("Welcome to TechHub!\nVisit {confirmation_link} to confirm your registration.",);
    let html_body = format!(
        "Welcome to TechHub!<br />\
        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription.",
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

impl std::fmt::Debug for UserActivationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for UserActivationError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            UserActivationError::UnknownToken => StatusCode::UNAUTHORIZED,
            UserActivationError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        build_error_response(status_code, self.to_string())
    }
}

#[tracing::instrument(
    skip_all,
    fields(user_id=tracing::field::Empty)
)]
pub async fn confirm_user_activation(
    parameters: web::Query<ActivationParameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, UserActivationError> {
    let user_id = get_user_id_from_token(&pool, &parameters.token)
        .await?
        // Domain error (invalid token), so a new `UserConfirmError::UnknownToken` error is created as there's no existing error to wrap in an `anyhow::Error`
        .ok_or(UserActivationError::UnknownToken)?;
    tracing::Span::current().record("user_id", tracing::field::display(user_id));

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
