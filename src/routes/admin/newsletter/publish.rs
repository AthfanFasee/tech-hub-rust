use std::fmt::{self, Debug, Formatter};

use actix_web::{HttpRequest, HttpResponse, ResponseError, http::StatusCode, web};
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::{NewsLetterData, Newsletter},
    idempotency,
    idempotency::{IdempotencyKey, NextAction},
    repository, utils,
};

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

impl Debug for PublishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

    let mut transaction =
        match idempotency::try_processing(&pool, &idempotency_key, *user_id).await? {
            NextAction::StartProcessing(t) => t,
            NextAction::ReturnSavedResponse(saved_response) => {
                return Ok(saved_response);
            }
        };

    let issue_id = repository::insert_newsletter_issue(
        &mut transaction,
        newsletter.title.as_ref(),
        newsletter.content.text.as_ref(),
        newsletter.content.html.as_ref(),
    )
    .await?;

    repository::enqueue_delivery_tasks(&mut transaction, issue_id).await?;

    let response = HttpResponse::Ok().finish();
    let response =
        idempotency::save_response(transaction, &idempotency_key, *user_id, response).await?;
    Ok(response)
}
