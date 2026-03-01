use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{Comment, CommentRecord, CommentResponseBody},
    routes::CommentError,
};

#[tracing::instrument(skip(pool), fields(post_id=%post_id))]
pub async fn get_comments_for_post(
    post_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<CommentResponseBody>, anyhow::Error> {
    let rows = sqlx::query_as::<_, CommentRecord>(
        r#"
        SELECT c.id, c.text, c.created_by, c.post_id, u.user_name AS user_name, c.created_at
        FROM comments c
        INNER JOIN users u ON c.created_by = u.id
        WHERE post_id = $1
        ORDER BY c.id DESC
        "#,
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
    .context("Failed to load comments for posts")?;

    let comments = rows.into_iter().map(CommentResponseBody::from).collect();

    Ok(comments)
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

#[tracing::instrument(skip(pool), fields(comment_id=%id))]
pub async fn delete_comment(id: Uuid, pool: &PgPool) -> Result<(), CommentError> {
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

#[tracing::instrument(skip(pool))]
pub async fn did_user_create_the_comment(
    comment_id: Uuid,
    user_id: Uuid,
    pool: &PgPool,
) -> Result<bool, CommentError> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM comments
            WHERE id = $1
            AND created_by = $2
        ) AS "exists!"
        "#,
        comment_id,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to check if user created this comment")?;

    Ok(result)
}
