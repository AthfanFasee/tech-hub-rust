use crate::authentication::UserId;
use crate::domain::{CreatedBy, Filters, GetAllPostsQuery, Img, Limit, Metadata, Page, Post, PostRecord, PostResponse, QueryTitle, Sort, SortDirection, Text, Title};
use crate::{build_error_response, error_chain_fmt};
use actix_web::http::StatusCode;
use actix_web::ResponseError;
use actix_web::{web, HttpResponse};
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

#[tracing::instrument(
    skip(pool, query),
    fields(
        title = %query.title,
        page = %query.page,
        limit = %query.limit,
        sort = %query.sort,
        id = %query.id
    )
)]
pub async fn get_all_posts(
    query: web::Query<GetAllPostsQuery>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PostError> {
    let query = query.into_inner();
    // Parse and validate query parameters
    let title = if query.title.is_empty() {
        None
    } else {
        Some(QueryTitle::parse(query.title).map_err(PostError::ValidationError)?)
    };

    let created_by_id = if query.id.is_empty() {
        None
    } else {
        Some(CreatedBy::parse(query.id).map_err(PostError::ValidationError)?)
    };

    let page = Page::parse(query.page).map_err(PostError::ValidationError)?;
    let limit = Limit::parse(query.limit).map_err(PostError::ValidationError)?;
    let sort = Sort::parse(&query.sort).map_err(PostError::ValidationError)?;

    let filters = Filters { page, limit, sort };

    // Fetch posts and count
    let (posts, total_records) =
        fetch_posts_with_count(title.as_ref(), created_by_id.as_ref(), &filters, &pool).await?;

    // Calculate metadata
    let metadata = Metadata::calculate(total_records, filters.page.value(), filters.limit.value());

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "posts": posts,
        "metadata": metadata
    })))
}

#[tracing::instrument(skip(pool))]
async fn fetch_posts_with_count(
    title: Option<&QueryTitle>,
    created_by_id: Option<&CreatedBy>,
    filters: &Filters,
    pool: &PgPool,
) -> Result<(Vec<PostResponse>, i64), PostError> {
    let title_search = title.map(|t| t.as_ref().to_string()).unwrap_or_default();
    let offset = filters.offset() as i64;
    let limit = filters.limit.value() as i64;
    let sort_clause = filters.sort.to_sql();

    // Build WHERE clause conditionally based on created_by_id
    let (where_clause, params_count) = if created_by_id.is_some() {
        (
            "WHERE (to_tsvector('english', title) @@ plainto_tsquery('english', $1) OR $1 = '')
        AND p.created_by = $2
        AND p.deleted_at IS NULL",
            2,
        )
    } else {
        (
            "WHERE (to_tsvector('english', title) @@ plainto_tsquery('english', $1) OR $1 = '')
        AND p.deleted_at IS NULL",
            1,
        )
    };

    let query = format!(
        r#"
        SELECT COUNT(*) OVER() AS total_count,
               p.id, p.title, p.post_text, p.img, p.version,
               p.liked_by, p.created_by, p.created_at, u.user_name as created_by_name
        FROM posts p
        INNER JOIN users u ON p.created_by = u.id
        {}
        ORDER BY {}, p.created_at {}
        LIMIT ${} OFFSET ${}
        "#,
        where_clause,
        sort_clause,
        match filters.sort.direction {
            SortDirection::Desc => "DESC",
            SortDirection::Asc => "ASC",
        },
        params_count + 1,
        params_count + 2
    );

    let mut query_builder = sqlx::query_as::<_, PostRecord>(&query).bind(&title_search);

    if let Some(creator_id) = created_by_id {
        query_builder = query_builder.bind(creator_id.as_ref());
    }

    let records = query_builder
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to fetch posts from database")?;

    let total_count = records.first().map(|r| r.total_count).unwrap_or(0);

    let posts = records
        .into_iter()
        .map(|record| PostResponse {
            id: record.id,
            title: record.title,
            text: record.post_text,
            img: record.img,
            version: record.version,
            created_at: record.created_at,
            created_by: record.created_by,
            liked_by: record.liked_by.unwrap_or_default(),
        })
        .collect();

    Ok((posts, total_count))
}

#[derive(Deserialize, Debug)]
pub struct PostPathParams {
    pub id: Uuid,
}

#[tracing::instrument(
    skip_all,
    fields(post_id = %path.id)
)]
pub async fn get_post(
    path: web::Path<PostPathParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PostError> {
    let post_id = path.id;

    let post = get_post_by_id(post_id, &pool).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"post": post})))
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
pub struct UpdatePostPayload {
    pub title: String,
    pub text: String,
    pub img: String,
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
    path: web::Path<PostPathParams>,
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

async fn get_post_by_id(id: Uuid, pool: &PgPool) -> Result<PostResponse, PostError> {
    let record = sqlx::query!(
        r#"
        SELECT id, title, post_text, img, version, created_at, created_by, liked_by
        FROM posts
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        id
    )
        .fetch_optional(pool)
        .await
        .context("Failed to fetch post from database")?;

    match record {
        Some(rec) => Ok(PostResponse {
            id: rec.id,
            title: rec.title,
            text: rec.post_text,
            img: rec.img,
            version: rec.version,
            created_at: rec.created_at,
            created_by: rec.created_by,
            liked_by: rec.liked_by,
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
    path: web::Path<PostPathParams>,
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
    path: web::Path<PostPathParams>,
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

    let post = get_post_by_id(post_id, &pool).await?;

    add_like_to_post(post_id, *user_id, &pool).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "post": post })))
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

    let post = get_post_by_id(post_id, &pool).await?;

    remove_like_from_post(post_id, *user_id, &pool).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "post": post })))
}

#[tracing::instrument(skip(pool), fields(post_id=%post_id, user_id=%user_id))]
async fn add_like_to_post(post_id: Uuid, user_id: Uuid, pool: &PgPool) -> Result<(), PostError> {
    // unnest() converts an array into a set of rows (like a table column).
    // t(x) means "create a temporary table t with one column x holding each value from the array."
    // `array_agg(DISTINCT x)` takes all those rows and aggregate them back into an array using DISTINCT to remove duplicates.
    let result = sqlx::query!(
        r#"
        UPDATE posts
        SET liked_by = (
            SELECT array_agg(DISTINCT x)
            FROM unnest(array_append(liked_by, $1)) t(x)
        )
        WHERE id = $2 AND deleted_at IS NULL
        "#,
        user_id,
        post_id
    )
        .execute(pool)
        .await
        .context("Failed to add like to post")?;

    if result.rows_affected() == 0 {
        return Err(PostError::NotFound);
    }

    Ok(())
}

#[tracing::instrument(skip(pool), fields(post_id=%post_id, user_id=%user_id))]
async fn remove_like_from_post(
    post_id: Uuid,
    user_id: Uuid,
    pool: &PgPool,
) -> Result<(), PostError> {
    let result = sqlx::query!(
        r#"
        UPDATE posts
        SET liked_by = array_remove(liked_by, $1)
        WHERE id = $2 AND deleted_at IS NULL
        "#,
        user_id,
        post_id
    )
        .execute(pool)
        .await
        .context("Failed to remove like from post")?;

    if result.rows_affected() == 0 {
        return Err(PostError::NotFound);
    }

    Ok(())
}


