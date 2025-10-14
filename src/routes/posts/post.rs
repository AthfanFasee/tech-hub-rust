use crate::authentication::UserId;
use crate::domain::{Img, Post, Text, Title};
use crate::{build_error_response, error_chain_fmt};
use actix_web::ResponseError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, web};
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum PostError {
    #[error("{0}")]
    ValidationError(String),

    #[error("post not found")]
    NotFound,

    #[error("edit conflict: post was modified by another request")]
    EditConflict,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PostError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            PostError::ValidationError(_) => StatusCode::BAD_REQUEST,
            PostError::NotFound => StatusCode::NOT_FOUND,
            PostError::EditConflict => StatusCode::CONFLICT,
            PostError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        build_error_response(status_code, self.to_string())
    }
}

#[derive(Deserialize, Debug)]
pub struct CreatePostPayload {
    title: String,
    text: String,
    img: String,
}

#[derive(Serialize)]
pub struct CreatePostResponseBody<'a> {
    pub id: Uuid,
    pub title: &'a str,
    pub post_text: &'a str,
    pub img: &'a str,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

impl TryFrom<CreatePostPayload> for Post {
    type Error = String;

    fn try_from(payload: CreatePostPayload) -> Result<Self, Self::Error> {
        let post = Self::new(payload.title, payload.text, payload.img)?;
        Ok(post)
    }
}
#[tracing::instrument(
    skip(pool),
    fields(user_id=%&*user_id)
)]
pub async fn create_post(
    payload: web::Json<CreatePostPayload>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PostError> {
    let user_id = user_id.into_inner();
    let post: Post = payload.0.try_into().map_err(PostError::ValidationError)?;

    let (id, created_at) =
        insert_post_and_return_inserted_data(&post.title, &post.text, &post.img, user_id, &pool)
            .await
            .context("Failed to insert post record")?;

    let response = CreatePostResponseBody {
        id,
        title: post.title.as_ref(),
        post_text: post.text.as_ref(),
        img: post.img.as_ref(),
        created_at,
        created_by: *user_id,
    };

    Ok(HttpResponse::Created().json(response))
}

#[tracing::instrument(
    skip_all,
    fields(post_id=tracing::field::Empty)
)]
pub async fn insert_post_and_return_inserted_data(
    title: &Title,
    text: &Text,
    img: &Img,
    created_by: UserId,
    pool: &PgPool,
) -> Result<(Uuid, DateTime<Utc>), anyhow::Error> {
    let record = sqlx::query!(
        r#"
        INSERT INTO posts (id, title, post_text, img, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, created_at
        "#,
        Uuid::new_v4(),
        title.as_ref(),
        text.as_ref(),
        img.as_ref(),
        *created_by,
    )
    .fetch_one(pool)
    .await
    .context("Failed to insert new post into database")?;
    tracing::Span::current().record("post_id", tracing::field::display(&record.id));
    Ok((record.id, record.created_at))
}

#[derive(Deserialize, Debug)]
pub struct PathParams {
    pub id: Uuid,
}

#[derive(Deserialize, Debug)]
pub struct UpdatePostPayload {
    pub title: String,
    pub text: String,
    pub img: String,
}

#[derive(Serialize, Debug)]
pub struct PostData {
    pub id: Uuid,
    pub title: String,
    pub text: String,
    pub img: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    #[serde(default)]
    pub liked_by: Vec<i32>,
}

impl TryFrom<UpdatePostPayload> for Post {
    type Error = String;

    fn try_from(value: UpdatePostPayload) -> Result<Self, Self::Error> {
        Post::new(value.title, value.text, value.img)
    }
}

#[tracing::instrument(
    skip(pool),
    fields(post_id=%path.id)
)]
pub async fn update_post(
    path: web::Path<PathParams>,
    payload: web::Json<UpdatePostPayload>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;
    let validated_post: Post = payload.0.try_into().map_err(PostError::ValidationError)?;
    let mut post = get_post_by_id(post_id, &pool).await?;

    update_post_in_db(
        post.id,
        &validated_post.title,
        &validated_post.text,
        &validated_post.img,
        post.version,
        &pool,
    )
    .await?;

    post.title = validated_post.title.as_ref().to_string();
    post.text = validated_post.text.as_ref().to_string();
    post.img = validated_post.img.as_ref().to_string();

    Ok(HttpResponse::Ok().json(serde_json::json!({ "post": post })))
}

async fn get_post_by_id(id: Uuid, pool: &PgPool) -> Result<PostData, PostError> {
    let record = sqlx::query!(
        r#"
        SELECT id, title, post_text, img, version, created_at, created_by, liked_by
        FROM posts
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to fetch post from database")?;

    match record {
        Some(rec) => Ok(PostData {
            id: rec.id,
            title: rec.title,
            text: rec.post_text,
            img: rec.img,
            version: rec.version,
            created_at: rec.created_at,
            created_by: rec.created_by,
            liked_by: rec.liked_by.unwrap_or_default(),
        }),
        None => Err(PostError::NotFound),
    }
}

#[tracing::instrument(skip_all, fields(post_id=%id))]
async fn update_post_in_db(
    id: Uuid,
    title: &Title,
    text: &Text,
    img: &Img,
    version: i32,
    pool: &PgPool,
) -> Result<(), PostError> {
    let result = sqlx::query!(
        r#"
        UPDATE posts
        SET title = $1, post_text = $2, img = $3, version = version + 1
        WHERE id = $4 AND version = $5
        "#,
        title.as_ref(),
        text.as_ref(),
        img.as_ref(),
        id,
        version
    )
    .execute(pool)
    .await
    .context("Failed to execute update query")?;

    if result.rows_affected() == 0 {
        return Err(PostError::EditConflict);
    }

    Ok(())
}

#[tracing::instrument(
    skip(pool),
    fields(post_id=%path.id)
)]
pub async fn delete_post(
    path: web::Path<PathParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;

    // Soft delete: mark deleted_at = now()
    let result = sqlx::query!(
        r#"
        UPDATE posts
        SET deleted_at = $1
        WHERE id = $2 AND deleted_at IS NULL
        "#,
        Utc::now(),
        post_id
    )
    .execute(&**pool)
    .await
    .context("Failed to mark post as deleted")?;

    if result.rows_affected() == 0 {
        return Err(PostError::NotFound);
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    skip(pool),
    fields(post_id=%path.id)
)]
pub async fn hard_delete_post(
    path: web::Path<PathParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;

    // Soft delete: mark deleted_at = now()
    let result = sqlx::query!(
        r#"
        DELETE FROM posts
	    WHERE id = $1
        "#,
        post_id
    )
    .execute(&**pool)
    .await
    .context("Failed to hard delete post")?;

    if result.rows_affected() == 0 {
        return Err(PostError::NotFound);
    }

    Ok(HttpResponse::Ok().finish())
}
