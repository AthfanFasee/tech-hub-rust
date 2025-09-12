use crate::domain::{NewUser, UserEmail, UserName};
use crate::email_client::EmailClient;
use crate::email_client::EmailError;
use crate::startup::ApplicationBaseUrl;
use actix_web::ResponseError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, web};
use anyhow::Context;
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use serde::Deserialize;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UserData {
    email: String,
    name: String,
}

// This is like saying - I know how to build myself `NewUser` from something else `UserData`
// Then Rust lets us use `.try_into` whenever there's a `UserData` - where it automatically tries converting it to a `NewUser`
impl TryFrom<UserData> for NewUser {
    type Error = String;

    fn try_from(payload: UserData) -> Result<Self, Self::Error> {
        let name = UserName::parse(payload.name)?;
        let email = UserEmail::parse(payload.email)?;
        Ok(Self { name, email })
    }
}

pub fn error_chain_fmt(
    e: &(dyn std::error::Error),
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    // Top-level: use Display to avoid recursion
    writeln!(f, "{e}")?;

    let mut current = e.source();
    while let Some(cause) = current {
        // For causes: use Debug if caller asked for `:#?` (`tracing::debug!("{:#?}", err)`), else Display (`tracing::error!("{:?}", err)`)
        if f.alternate() {
            writeln!(f, "Caused by:\n\t{cause:?}")?;
        } else {
            writeln!(f, "Caused by:\n\t{cause}")?;
        }
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum UserError {
    // the 0 is something like `self.0` and will print the String value the ValidationError wraps around
    #[error("{0}")]
    ValidationError(String),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for UserError {
    fn status_code(&self) -> StatusCode {
        match self {
            UserError::ValidationError(_) => StatusCode::BAD_REQUEST,
            UserError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(
    name = "Add a new user",
    skip(pool, payload, email_client, base_url),
    fields(
        user_email = %payload.email,
        user_name = %payload.name,
    )
)]
pub async fn add_user(
    payload: web::Json<UserData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, UserError> {
    let new_user = payload.0.try_into().map_err(UserError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let user_id = insert_user(&new_user, &mut transaction)
        .await
        .context("Failed to insert new user in the database")?;

    let activation_token = generate_token();

    store_activation_token(&mut transaction, user_id, &activation_token)
        .await
        .context("Failed to store the confirmation token for new user")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new user")?;

    send_confirmation_email(&email_client, new_user, &base_url.0, &activation_token)
        .await
        .context("Failed to send a confirmation email when registering new user")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Save new user details in the database",
    skip(new_user, transaction)
)]
pub async fn insert_user(
    new_user: &NewUser,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let user_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
            INSERT INTO users (id, name, email, password_hash)
            VALUES ($1, $2, $3, $4)
           "#,
        user_id,
        new_user.name.as_ref(),
        new_user.email.as_ref(),
        "dummy_hash",
    );

    transaction.execute(query).await?;
    Ok(user_id)
}
#[tracing::instrument(name = "Store token in the database", skip(token, transaction))]
pub async fn store_activation_token(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO tokens (token, user_id, is_activation)
            VALUES ($1, $2, $3)"#,
        token,
        user_id,
        true,
    );

    transaction.execute(query).await?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to new user",
    skip(email_client, new_user)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_user: NewUser,
    base_url: &str,
    token: &str,
) -> Result<(), EmailError> {
    let confirmation_link = format!("{base_url}/user/confirm?token={token}");
    let plain_body =
        format!("Welcome to Moodfeed!\nVisit {confirmation_link} to confirm your subscription.",);
    let html_body = format!(
        "Welcome to Moodfeed!<br />\
        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription.",
    );
    email_client
        .send_email(new_user.email, "Welcome!", &html_body, &plain_body)
        .await
}

// Generate a random 25-characters-long case-sensitive token.
fn generate_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
