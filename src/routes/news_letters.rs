use crate::domain::UserEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;
use crate::routes::ErrorResponse;
use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::http::{header, StatusCode};
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use secrecy::ExposeSecret;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                let status_code = StatusCode::INTERNAL_SERVER_ERROR;
                let error_response = ErrorResponse {
                    code: status_code.as_u16(),
                    message: self.to_string(),
                };
                HttpResponse::build(status_code).json(error_response)
            }
            PublishError::AuthError(_) => {
                let status_code = StatusCode::UNAUTHORIZED;
                let error_response = ErrorResponse {
                    code: status_code.as_u16(),
                    message: self.to_string(),
                };

                let mut response = HttpResponse::build(status_code).json(error_response);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}
#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]

pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    let users = get_activated_users(&pool).await?;
    for user in users {
        match user {
            Ok(user) => {
                let user_email = &user.email;
                email_client
                    .send_email(
                        user_email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| format!("Failed to send newsletter issue to {user_email}"))?;
            }

            Err(error) => {
                tracing::warn!(
                    // Create a structured log field called error.cause_chain.
                    // Format the variable error with Debug ({:?}), which for anyhow::Error prints the error and its cause chain.
                    error.cause_chain = ?error,
                    "Skipping a confirmed user. \
                    Their stored contact details are invalid",
                );
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

struct ConfirmedUser {
    email: UserEmail,
}

#[tracing::instrument(name = "Get activated users", skip(pool))]
async fn get_activated_users(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedUser, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email
        FROM users
        WHERE is_activated = true
        "#,
    )
        .fetch_all(pool)
        .await?;
    let confirmed_users = rows
        .into_iter()
        .map(|r| match UserEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedUser { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_users)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    // The header value, if present, must be a valid UTF8 string
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    // Split into two segments, using ':' as delimiter
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();
    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let (user_id, expected_password_hash) = get_stored_credentials(
        &credentials.username,
        pool,
    )
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| {
            PublishError::AuthError(anyhow::anyhow!("Unknown username."))
        })?;

    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishError::UnexpectedError)?;
    Argon2::default()
        .verify_password(
            credentials.password.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(PublishError::AuthError)?;
    Ok(user_id)
}


#[tracing::instrument(name = "Get stored credentials", skip(pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
    r#"
    SELECT id, password_hash
    FROM users
    WHERE name = $1
    "#,
    username,
    )
        .fetch_optional(pool)
        .await
        .context("Failed to perform a query to retrieve stored credentials.")?
        .map(|row| (row.id, Secret::new(row.password_hash)));
    Ok(row)
}