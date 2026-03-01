use anyhow::Context;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::PgTransaction;
use crate::domain::NewsletterIssue;

#[tracing::instrument(skip_all)]
pub async fn insert_newsletter_issue(
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
pub async fn enqueue_delivery_tasks(
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

pub async fn get_newsletter_issue(
    transaction: &mut PgTransaction,
    issue_id: Uuid,
) -> Result<NewsletterIssue, anyhow::Error> {
    let row = sqlx::query!(
        r#"
    SELECT title, text_content, html_content
    FROM newsletter_issues
    WHERE id = $1
    "#,
        issue_id
    )
    .fetch_one(&mut **transaction)
    .await
    .context("Failed to get newsletter issue details")?;

    Ok(NewsletterIssue::new(
        row.title,
        row.text_content,
        row.html_content,
    ))
}

// Moving to an archive table rather than deleting would be preferable if you want to record keep
#[tracing::instrument(skip(pool))]
pub async fn cleanup_old_newsletter_issues(pool: &PgPool) -> Result<(), anyhow::Error> {
    let deleted = sqlx::query!(
        r#"
        DELETE FROM newsletter_issues
        WHERE created_at < NOW() - INTERVAL '7 days'
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    tracing::info!(deleted, "Old newsletter issues cleanup completed");
    Ok(())
}
