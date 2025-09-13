use crate::domain::UserEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use actix_web::http::StatusCode;
use actix_web::web;
use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
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
                    "Skipping a confirmed subscriber. \
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
