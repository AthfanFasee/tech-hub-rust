use crate::authentication::UserId;
use crate::domain::Comment;
use crate::{build_error_response, error_chain_fmt};
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error)]
pub enum CommentError {
    #[error("{0}")]
    ValidationError(String),

    #[error("comment not found")]
    NotFound,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for CommentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for CommentError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            CommentError::ValidationError(_) => actix_web::http::StatusCode::BAD_REQUEST,
            CommentError::NotFound => actix_web::http::StatusCode::NOT_FOUND,
            CommentError::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        build_error_response(status_code, self.to_string())
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

    let comments = get_comments_for_post(post_id, &pool)
        .await
        .map_err(CommentError::UnexpectedError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "comments": comments })))
}

#[tracing::instrument(skip(pool), fields(post_id=%post_id))]
pub async fn get_comments_for_post(
    post_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<CommentResponseBody>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT c.id, c.text, c.created_by, c.post_id, u.user_name AS user_name, c.created_at
        FROM comments c
        INNER JOIN users u ON c.created_by = u.id
        WHERE post_id = $1
        ORDER BY c.id DESC
        "#,
        post_id
    )
    .fetch_all(pool)
    .await
    .context("Failed to load comments for post")?;

    let comments = rows
        .into_iter()
        .map(|r| CommentResponseBody {
            id: r.id,
            text: r.text,
            created_by: r.created_by,
            post_id: r.post_id,
            created_at: r.created_at,
        })
        .collect();

    Ok(comments)
}

#[derive(Deserialize, Debug)]
pub struct CreateCommentPayload {
    pub text: String,
    pub post_id: String,
}

impl TryFrom<CreateCommentPayload> for Comment {
    type Error = String;

    fn try_from(value: CreateCommentPayload) -> Result<Self, Self::Error> {
        Comment::new(value.text, value.post_id)
    }
}

#[derive(Serialize, Debug)]
pub struct CommentResponseBody {
    pub id: Uuid,
    pub text: String,
    pub post_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

#[tracing::instrument(skip_all)]
pub async fn create_comment(
    payload: web::Json<CreateCommentPayload>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, CommentError> {
    let user_id = user_id.into_inner();

    let comment: Comment = Comment::new(payload.text.clone(), payload.post_id.clone())
        .map_err(CommentError::ValidationError)?;

    let (id, created_at) = insert_comment(&comment, *user_id, &pool)
        .await
        .map_err(CommentError::UnexpectedError)?;

    let resp = CommentResponseBody {
        id,
        text: comment.text.as_ref().to_string(),
        post_id: comment.post_id,
        created_at,
        created_by: *user_id,
    };

    Ok(HttpResponse::Created().json(serde_json::json!(resp)))
}

#[tracing::instrument(skip(pool), fields(post_id=%comment.post_id))]
pub async fn insert_comment(
    comment: &Comment,
    user_id: Uuid,
    pool: &PgPool,
) -> Result<(Uuid, DateTime<Utc>), anyhow::Error> {
    let record = sqlx::query!(
        r#"
        INSERT INTO comments (id, text, post_id, created_by)
        VALUES ($1, $2, $3, $4)
        RETURNING id, created_at
        "#,
        Uuid::new_v4(),
        comment.text.as_ref(),
        comment.post_id,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to insert comment")?;

    Ok((record.id, record.created_at))
}

#[tracing::instrument(skip(pool), fields(comment_id=%path.id))]
pub async fn delete_comment(
    path: web::Path<CommentPathParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, CommentError> {
    let id = path.id;

    delete_comment_db(id, &pool).await?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(skip(pool), fields(comment_id=%id))]
pub async fn delete_comment_db(id: Uuid, pool: &PgPool) -> Result<(), CommentError> {
    let result = sqlx::query!(
        r#"
        DELETE FROM comments
        WHERE id = $1
        "#,
        id
    )
    .execute(pool)
    .await
    .context("Failed to delete comment")?;

    if result.rows_affected() == 0 {
        return Err(CommentError::NotFound);
    }

    Ok(())
}
