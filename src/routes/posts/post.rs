use std::fmt::{self, Debug, Formatter};

use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::Span;
use uuid::Uuid;

use crate::{
    authentication::{IsAdmin, UserId},
    domain::{
        CreatePostPayload, CreatePostResponse, GetAllPostsQuery, Metadata, Post, PostQuery,
        UpdatePostPayload,
    },
    repository, utils,
};

#[derive(thiserror::Error)]
pub enum PostError {
    #[error("{0}")]
    ValidationError(String),

    #[error("post not found")]
    NotFound,

    #[error("not authorized to perform this action")]
    Forbidden,

    #[error("edit conflict: posts was modified by another request")]
    EditConflict,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PostError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for PostError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            PostError::ValidationError(_) => StatusCode::BAD_REQUEST,
            PostError::NotFound => StatusCode::NOT_FOUND,
            PostError::Forbidden => StatusCode::FORBIDDEN,
            PostError::EditConflict => StatusCode::CONFLICT,
            PostError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        utils::build_error_response(status_code, self.to_string())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn get_all_posts(
    query: web::Query<GetAllPostsQuery>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PostError> {
    let parsed_query =
        PostQuery::try_from(query.into_inner()).map_err(PostError::ValidationError)?;

    let (posts, total_records) = repository::get_all_posts(
        parsed_query.title.as_ref(),
        parsed_query.created_by_id.as_ref(),
        &parsed_query.filters,
        &pool,
    )
    .await?;

    let metadata = Metadata::calculate(
        total_records,
        parsed_query.filters.page.value(),
        parsed_query.filters.limit.value(),
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "posts": posts,
        "metadata": metadata
    })))
}

#[derive(Deserialize, Debug)]
pub struct PostPathParams {
    pub id: Uuid,
}

pub async fn get_post(
    path: web::Path<PostPathParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;

    let post = repository::get_post(post_id, &pool).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"posts": post})))
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
        repository::insert_post(&post.title, &post.text, &post.img, user_id, &pool)
            .await
            .context("Failed to insert posts record")?;

    let response = CreatePostResponse {
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
    skip(pool),
    fields(user_id=tracing::field::Empty, post_id=%path.id)
)]
pub async fn update_post(
    path: web::Path<PostPathParams>,
    payload: web::Json<UpdatePostPayload>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    is_admin: web::ReqData<IsAdmin>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;
    let user_id = user_id.into_inner();
    let is_admin = *is_admin.into_inner();

    Span::current().record("user_id", tracing::field::display(&user_id));

    // If not admin, verify ownership
    if !is_admin {
        let is_owner = repository::did_user_create_the_post(post_id, *user_id, &pool).await?;

        if !is_owner {
            return Err(PostError::Forbidden);
        }
    }

    let validated_post: Post = payload.0.try_into().map_err(PostError::ValidationError)?;
    let mut post = repository::get_post(post_id, &pool).await?;

    repository::update_post(
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

    Ok(HttpResponse::Ok().json(serde_json::json!({ "posts": post })))
}

pub async fn delete_post(
    path: web::Path<PostPathParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    is_admin: web::ReqData<IsAdmin>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;
    let user_id = *user_id.into_inner();
    let is_admin = *is_admin.into_inner();

    // if not admin, then verify ownership
    if !is_admin {
        let is_owner = repository::post::did_user_create_the_post(post_id, user_id, &pool).await?;
        if !is_owner {
            return Err(PostError::Forbidden);
        }
    }

    let deleted = repository::post::soft_delete_post(post_id, &pool).await?;
    if !deleted {
        return Err(PostError::NotFound);
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    skip(pool, user_id),
    fields(post_id=%path.id, user_id=%&*user_id)
)]
pub async fn like_post(
    path: web::Path<PostPathParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;
    let user_id = user_id.into_inner();

    let post = repository::get_post(post_id, &pool).await?;

    repository::add_like_to_post(post_id, *user_id, &pool).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "posts": post })))
}

#[tracing::instrument(
    skip(pool, user_id),
    fields(post_id=%path.id, user_id=%&*user_id)
)]
pub async fn dislike_post(
    path: web::Path<PostPathParams>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;
    let user_id = user_id.into_inner();

    let post = repository::get_post(post_id, &pool).await?;

    repository::remove_like_from_post(post_id, *user_id, &pool).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "posts": post })))
}
