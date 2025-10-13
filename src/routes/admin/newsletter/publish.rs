use crate::authentication::UserId;
use crate::idempotency::{IdempotencyKey, NextAction};
use crate::idempotency::{save_response, try_processing};
use crate::{build_error_response, error_chain_fmt};
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, ResponseError, web};
use anyhow::Context;
use serde::Deserialize;
use sqlx::{Executor, PgPool};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Invalid request: {0}")]
    BadRequest(#[source] anyhow::Error),
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
        let status_code = match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PublishError::AuthError(_) => StatusCode::UNAUTHORIZED,
            PublishError::BadRequest(_) => StatusCode::BAD_REQUEST,
        };

        build_error_response(status_code, self.to_string())
    }
}

#[derive(Deserialize)]
pub struct NewsLetterData {
    title: String,
    content: Content,
}
#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
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

    let idempotency_key = req
        .headers()
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let idempotency_key: IdempotencyKey = idempotency_key
        .try_into()
        .map_err(PublishError::BadRequest)?;

    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id).await? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(
        &mut transaction,
        &payload.title,
        &payload.content.text,
        &payload.content.html,
    )
    .await?;

    enqueue_delivery_tasks(&mut transaction, issue_id).await?;

    let response = HttpResponse::Ok().finish();
    let response = save_response(transaction, &idempotency_key, *user_id, response).await?;
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
        newsletter_issue_id,
        title,
        text_content,
        html_content,
        published_at
        )
        VALUES ($1, $2, $3, $4, now())
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
