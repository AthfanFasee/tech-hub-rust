use crate::authentication::UserId;
use crate::domain::{NewsLetterData, Newsletter};
use crate::idempotency::{IdempotencyKey, NextAction};
use crate::idempotency;
use crate::utils;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, ResponseError, web};
use anyhow::Context;
use sqlx::{Executor, PgPool};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("{0}")]
    ValidationError(String),

    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),

    #[error("Invalid request: {0}")]
    BadRequest(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            PublishError::ValidationError(_) => StatusCode::BAD_REQUEST,
            PublishError::AuthError(_) => StatusCode::UNAUTHORIZED,
            PublishError::BadRequest(_) => StatusCode::BAD_REQUEST,
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}

#[tracing::instrument(
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn publish_newsletter(
    req: HttpRequest,
    payload: web::Json<NewsLetterData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PublishError> {
    let user_id = user_id.into_inner();

    let newsletter: Newsletter = payload
        .0
        .try_into()
        .map_err(PublishError::ValidationError)?;

    let idempotency_key = req
        .headers()
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let idempotency_key: IdempotencyKey = idempotency_key
        .try_into()
        .map_err(PublishError::BadRequest)?;

    let mut transaction = match idempotency::try_processing(&pool, &idempotency_key, *user_id).await? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(
        &mut transaction,
        newsletter.title.as_ref(),
        newsletter.content.text.as_ref(),
        newsletter.content.html.as_ref(),
    )
    .await?;

    enqueue_delivery_tasks(&mut transaction, issue_id).await?;

    let response = HttpResponse::Ok().finish();
    let response = idempotency::save_response(transaction, &idempotency_key, *user_id, response).await?;
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, anyhow::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
        id,
        title,
        text_content,
        html_content
        )
        VALUES ($1, $2, $3, $4)
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    );
    transaction
        .execute(query)
        .await
        .context("Failed to store newsletter issue details")?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip(transaction))]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
        newsletter_issue_id,
        user_email
        )
        SELECT $1, email
        FROM users
        WHERE is_activated = true and is_subscribed = true
        "#,
        newsletter_issue_id,
    );
    transaction
        .execute(query)
        .await
        .context("Failed to enqueue delivery tasks")?;
    Ok(())
}
