use std::fmt::{self, Debug, Formatter};

use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use serde::Deserialize;
use sqlx::PgPool;
use thiserror;
use uuid::Uuid;

use crate::{
    authentication::{IsAdmin, UserId},
    domain::{Comment, CreateCommentPayload, CreateCommentResponseBody},
    repository, utils,
};

#[derive(thiserror::Error)]
pub enum CommentError {
    #[error("{0}")]
    ValidationError(String),

    #[error("comment not found")]
    NotFound,

    #[error("not authorized to perform this action")]
    Forbidden,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for CommentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for CommentError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            CommentError::ValidationError(_) => StatusCode::BAD_REQUEST,
            CommentError::NotFound => StatusCode::NOT_FOUND,
            CommentError::Forbidden => StatusCode::FORBIDDEN,
            CommentError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}

#[derive(Deserialize, Debug)]
pub struct CommentPathParams {
    pub id: Uuid,
}

#[tracing::instrument(skip(pool), fields(post_id=%path.id))]
pub async fn show_comments_for_post(
    path: web::Path<CommentPathParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, CommentError> {
    let post_id = path.id;

    let comments = repository::get_comments_for_post(post_id, &pool)
        .await
        .map_err(CommentError::UnexpectedError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "comments": comments })))
}

#[tracing::instrument(skip(pool), fields(user_id=%&*user_id))]
pub async fn create_comment(
    payload: web::Json<CreateCommentPayload>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, CommentError> {
    let user_id = user_id.into_inner();

    let comment: Comment = payload
        .0
        .try_into()
        .map_err(CommentError::ValidationError)?;

    let (id, created_at) = repository::insert_comment(&comment, *user_id, &pool)
        .await
        .map_err(CommentError::UnexpectedError)?;

    let resp = CreateCommentResponseBody {
        id,
        text: comment.text.as_ref(),
        post_id: comment.post_id,
        created_at,
        created_by: *user_id,
    };

    Ok(HttpResponse::Created().json(resp))
}

#[tracing::instrument(skip(pool), fields(comment_id=%path.id))]
pub async fn delete_comment(
    path: web::Path<CommentPathParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    is_admin: web::ReqData<IsAdmin>,
) -> Result<HttpResponse, CommentError> {
    let comment_id = path.id;
    let user_id = user_id.into_inner();
    let is_admin = *is_admin.into_inner();

    // If not admin, verify ownership
    if !is_admin {
        let is_owner = repository::did_user_create_the_comment(comment_id, *user_id, &pool).await?;

        if !is_owner {
            return Err(CommentError::Forbidden);
        }
    }

    repository::delete_comment(comment_id, &pool).await?;
    Ok(HttpResponse::Ok().finish())
}
