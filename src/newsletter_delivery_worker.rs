use crate::domain::UserEmail;
use crate::email_client::EmailClient;
use crate::{configuration::Configuration, startup::get_connection_pool};
use anyhow::Context;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use std::ops::DerefMut;
use tokio::time::Duration;
use tracing::{Span, field::display};
use uuid::Uuid;

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

pub async fn run_worker_until_stopped(config: Configuration) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&config.database);
    let email_client = config.email_client.client();
    worker_loop(connection_pool, email_client).await
}

async fn worker_loop(pool: PgPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    // spawn idempotency data cleanup loop independently
    let pool_for_cleanup = pool.clone();
    tokio::spawn(async move {
        let mut rng = StdRng::from_entropy();
        loop {
            if let Err(e) = cleanup_old_idempotency_records(&pool_for_cleanup).await {
                tracing::error!(error = ?e, "Idempotency cleanup failed");
            }

            // This random jitter will ensure multiple instances of app won't clean db at same time
            // Nonetheless a delete statement is concurrency safe in db
            let jitter = rng.gen_range(0..=3600);
            tokio::time::sleep(Duration::from_secs(12 * 3600 + jitter)).await;
        }
    });

    let mut rng = StdRng::from_entropy();
    // start with 1s base delay, max 1 minute
    let mut backoff_secs = 1_u64;

    // newsletter dispatch worker loop
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                // Zero pending tasks hence sleep longer, reset backoff
                backoff_secs = 1;
                tokio::time::sleep(Duration::from_secs(600)).await;
            }

            Ok(ExecutionOutcome::TaskCompleted) => {
                // success hence reset backoff
                backoff_secs = 1;
            }

            Err(e) => {
                tracing::warn!(error = ?e, "Transient failure while executing task");
                // Add 0–20% random jitter to avoid sync storms
                let jitter = rng.gen_range(0.0..=0.2);
                let sleep_duration = Duration::from_secs_f64(backoff_secs as f64 * (1.0 + jitter));
                tokio::time::sleep(sleep_duration).await;

                // exponential backoff, capped at 60s
                backoff_secs = (backoff_secs * 2).min(60);
            }
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty
    ),
)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    let task = dequeue_task(pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }
    let (transaction, issue_id, email) = task.unwrap();

    Span::current()
        .record("newsletter_issue_id", display(issue_id))
        .record("subscriber_email", display(&email));

    match UserEmail::parse(email.clone()) {
        Ok(email) => {
            let issue = get_newsletter_issue(pool, issue_id).await?;
            if let Err(e) = email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Failed to deliver news letter issue to a subscribed user. \
                Skipping.",
                );
            }
        }
        Err(e) => {
            tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "Skipping a subscribed user. \
            Their stored contact details are invalid",
            );
        }
    }
    delete_task(transaction, issue_id, &email).await?;
    Ok(ExecutionOutcome::TaskCompleted)
}

type PgTransaction = Transaction<'static, Postgres>;

async fn dequeue_task(
    pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to start a transaction")?;
    let r = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, user_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
    )
    .fetch_optional(transaction.deref_mut())
    .await
    .context("Failed dequeue a newsletter issue task from db")?;

    if let Some(r) = r {
        Ok(Some((transaction, r.newsletter_issue_id, r.user_email)))
    } else {
        Ok(None)
    }
}
#[tracing::instrument(skip(transaction, email))]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
    DELETE FROM issue_delivery_queue
    WHERE
    newsletter_issue_id = $1 AND
    user_email = $2
    "#,
        issue_id,
        email
    );
    transaction
        .execute(query)
        .await
        .context("Failed delete a newsletter issue task from db")?;
    transaction
        .commit()
        .await
        .context("Failed to commit a transaction")?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

async fn get_newsletter_issue(
    pool: &PgPool,
    issue_id: Uuid,
) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
    SELECT title, text_content, html_content
    FROM newsletter_issues
    WHERE
    id = $1
    "#,
        issue_id
    )
    .fetch_one(pool)
    .await
    .context("Failed get a newsletter issue details")?;
    Ok(issue)
}

#[tracing::instrument(skip(pool))]
pub async fn cleanup_old_idempotency_records(pool: &PgPool) -> Result<(), anyhow::Error> {
    let deleted =
        sqlx::query!(r#"DELETE FROM idempotency WHERE created_at < NOW() - INTERVAL '24 hours'"#)
            .execute(pool)
            .await?
            .rows_affected();

    tracing::info!(deleted, "Idempotency cleanup completed");
    Ok(())
}
