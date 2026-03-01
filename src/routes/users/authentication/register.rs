use std::fmt::{self, Debug, Formatter};

use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use anyhow::Context;
use sqlx::PgPool;
use tracing::{Span, field};

use crate::{
    authentication,
    domain::{NewUser, UserData, UserEmail},
    email_client::{EmailClient, EmailError},
    repository,
    startup::ApplicationBaseUrl,
    telemetry, utils,
};

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

    let password_hash = telemetry::spawn_blocking_with_tracing(move || {
        authentication::compute_password_hash(password.into_secret())
    })
    .await
    .context("Failed to spawn blocking task")?
    .context("Failed to hash password")?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let user_id = repository::insert_user(&name, &email, password_hash, &mut transaction).await?;

    let activation_token = utils::generate_token();

    repository::store_activation_token(&mut transaction, user_id, &activation_token).await?;

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
    let user_id = repository::get_user_id_from_token(&pool, &parameters.token)
        .await?
        // Domain error (invalid token), so a new `UserConfirmError::UnknownToken` error is created as there's no existing error to wrap in an `anyhow::Error`
        .ok_or(UserActivationError::UnknownToken)?;
    Span::current().record("user_id", field::display(user_id));

    repository::activate_user_and_delete_token(&pool, user_id, &parameters.token).await?;
    Ok(HttpResponse::Ok().finish())
}
