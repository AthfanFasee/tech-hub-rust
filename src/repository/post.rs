use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::Span;
use uuid::Uuid;

use crate::{
    authentication::UserId,
    domain::{
        CreatedBy, Filters, PostImg, PostRecord, PostResponse, PostText, PostTitle, QueryTitle,
        SortDirection,
    },
    routes::PostError,
};

#[tracing::instrument(skip(pool))]
pub async fn get_all_posts(
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
        SELECT COUNT(*) OVER()::BIGINT AS total_count,
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
        .context("Failed to fetch posts")?;

    let total_count = records.first().map(|r| r.total_count).unwrap_or(0);

    let posts = records.into_iter().map(PostResponse::from).collect();

    Ok((posts, total_count))
}

pub async fn get_post(id: Uuid, pool: &PgPool) -> Result<PostResponse, PostError> {
    let record = sqlx::query_as::<_, PostRecord>(
        r#"
        SELECT 0::BIGINT as total_count, p.id, p.title, p.post_text, p.img, p.version, p.liked_by, p.created_by, p.created_at, u.user_name as created_by_name
        FROM posts p
        INNER JOIN users u ON p.created_by = u.id
        WHERE p.id = $1 AND deleted_at IS NULL
        "#,
    )
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch posts")?;

    match record {
        Some(rec) => Ok(PostResponse::from(rec)),
        None => Err(PostError::NotFound),
    }
}

#[tracing::instrument(
    skip_all,
    fields(post_id=tracing::field::Empty)
)]
pub async fn insert_post(
    title: &PostTitle,
    text: &PostText,
    img: &PostImg,
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
    .context("Failed to insert new posts")?;
    Span::current().record("post_id", tracing::field::display(&record.id));
    Ok((record.id, record.created_at))
}

#[tracing::instrument(skip_all, fields(post_id=%id))]
pub async fn update_post(
    id: Uuid,
    title: &PostTitle,
    text: &PostText,
    img: &PostImg,
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

#[tracing::instrument(skip(pool))]
pub async fn soft_delete_post(post_id: Uuid, pool: &PgPool) -> Result<bool, anyhow::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE posts
        SET deleted_at = $1
        WHERE id = $2 AND deleted_at IS NULL
        "#,
        Utc::now(),
        post_id
    )
    .execute(pool)
    .await
    .context("Failed to mark post as deleted")?;

    Ok(result.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn hard_delete_post(post_id: Uuid, pool: &PgPool) -> Result<bool, anyhow::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM posts
        WHERE id = $1
        "#,
        post_id
    )
    .execute(pool)
    .await
    .context("Failed to hard delete post")?;

    Ok(result.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn add_like_to_post(
    post_id: Uuid,
    user_id: Uuid,
    pool: &PgPool,
) -> Result<(), PostError> {
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
    .context("Failed to add like to posts")?;

    if result.rows_affected() == 0 {
        return Err(PostError::NotFound);
    }

    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn remove_like_from_post(
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
    .context("Failed to remove like from posts")?;

    if result.rows_affected() == 0 {
        return Err(PostError::NotFound);
    }

    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn did_user_create_the_post(
    post_id: Uuid,
    user_id: Uuid,
    pool: &PgPool,
) -> Result<bool, PostError> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM posts
            WHERE id = $1
            AND created_by = $2
            AND deleted_at IS NULL
        ) AS "exists!"
        "#,
        post_id,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to check if user created this post")?;

    Ok(result)
}
