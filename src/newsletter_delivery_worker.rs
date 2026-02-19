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
    // spawn cleanup loops independently
    let pool_for_cleanup = pool.clone();

    tokio::spawn(async move {
        let mut rng = StdRng::from_entropy();

        loop {
            if let Err(e) = cleanup_old_idempotency_records(&pool_for_cleanup).await {
                tracing::error!(error.cause_chain = ?e, "Idempotency cleanup failed");
            }
            if let Err(e) = cleanup_old_newsletter_issues(&pool_for_cleanup).await {
                tracing::error!(error.cause_chain = ?e, "Old newsletter cleanup failed");
            }

            // This random jitter will ensure multiple instances of app won't clean db at same time
            // Nonetheless a delete statement is concurrency safe in db
            let jitter = rng.gen_range(0..=3600);
            tokio::time::sleep(Duration::from_secs(24 * 3600 + jitter)).await;
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
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Transient failure while executing task"
                );

                // Add 0–20% random jitter to avoid sync storms
                let jitter = rng.gen_range(0.0..=0.2);
                let sleep_duration = Duration::from_secs_f64(backoff_secs as f64 * (1.0 + jitter));
                tokio::time::sleep(sleep_duration).await;

                // exponential backoff, capped at 120s
                backoff_secs = (backoff_secs * 2).min(120);
            }
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id = tracing::field::Empty,
        email = tracing::field::Empty
    ),
)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    // Fetch a pending delivery task (row locked until commit/rollback)
    let maybe_task = dequeue_task(pool).await?;
    if maybe_task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }

    let (mut transaction, issue_id, email, n_retries) = maybe_task.unwrap();

    Span::current()
        .record("newsletter_issue_id", display(issue_id))
        .record("subscriber_email", display(&email));

    // Process the task within the same transaction
    let result =
        process_delivery_task(&mut transaction, issue_id, &email, n_retries, email_client).await;

    match result {
        Ok(_) => {
            transaction
                .commit()
                .await
                .context("Failed to commit transaction after processing newsletter issue")?;
        }
        Err(e) => {
            // Try rollback
            if let Err(rb_err) = transaction.rollback().await {
                // If rollback failed combine both errors into one anyhow error
                let combined_error = anyhow::anyhow!(
                    "Task failed and rollback also failed.\n\
                Task error: {:#}\n\
                Rollback error: {:#}",
                    e,
                    rb_err
                );
                return Err(combined_error.context("Critical failure during newsletter delivery"));
            }

            // Rollback succeeded, return only the task error
            return Err(e.context("Task failed while processing newsletter delivery"));
        }
    }

    Ok(ExecutionOutcome::TaskCompleted)
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id = %issue_id,
        email = %email
    ),
)]
async fn process_delivery_task(
    transaction: &mut PgTransaction,
    issue_id: Uuid,
    email: &str,
    n_retries: i32,
    email_client: &EmailClient,
) -> Result<(), anyhow::Error> {
    let Ok(valid_email) = UserEmail::parse(email.to_string()) else {
        tracing::error!(
            %email,
            "Invalid subscriber email — deleting newsletter issue task permanently"
        );
        delete_task(transaction, issue_id, email).await?;
        return Ok(());
    };

    // Fetch issue content
    let issue = get_newsletter_issue(transaction, issue_id).await?;

    // Try sending the email
    match email_client
        .send_email(
            &valid_email,
            &issue.title,
            &issue.html_content,
            &issue.text_content,
        )
        .await
    {
        Ok(_) => {
            // success, remove from queue
            delete_task(transaction, issue_id, email).await?;
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Failed to deliver newsletter, will retry later."
            );
            retry_task(transaction, issue_id, email, n_retries).await?;
        }
    }

    Ok(())
}

type PgTransaction = Transaction<'static, Postgres>;

async fn dequeue_task(
    pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String, i32)>, anyhow::Error> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to start a transaction")?;

    let r = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, user_email, n_retries
        FROM issue_delivery_queue
        WHERE execute_after <= NOW()
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#
    )
    .fetch_optional(transaction.deref_mut())
    .await
    .context("Failed dequeue a newsletter issue task from db")?;

    if let Some(r) = r {
        Ok(Some((
            transaction,
            r.newsletter_issue_id,
            r.user_email,
            r.n_retries,
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id = %issue_id,
        email = %email
    ),
)]
async fn retry_task(
    transaction: &mut PgTransaction,
    issue_id: Uuid,
    email: &str,
    current_retry: i32,
) -> Result<(), anyhow::Error> {
    let next_retry = current_retry + 1;

    // give up after 5 attempts
    if next_retry > 5 {
        tracing::error!(%issue_id, "Max retries reached, dropping newsletter issue task permanently");
        delete_task(transaction, issue_id, email).await?;
        return Ok(());
    }

    // Exponential backoff: 1m, 2m, 4m, 8m, 16m, 32m, 60m
    let base_delay_secs = 60 * (1 << (next_retry - 1)).min(60);
    let jitter_secs: i64 = rand::thread_rng().gen_range(0..=30);
    let total_delay_secs = (base_delay_secs + jitter_secs) as f64;

    let query = sqlx::query!(
        r#"
        UPDATE issue_delivery_queue
        SET n_retries = $3,
            execute_after = NOW() + ($4 * INTERVAL '1 second')
        WHERE newsletter_issue_id = $1 AND user_email = $2
        "#,
        issue_id,
        email,
        next_retry,
        total_delay_secs
    );
    transaction
        .execute(query)
        .await
        .context("Failed to update a newsletter issue task with retry later info")?;

    Ok(())
}

async fn delete_task(
    transaction: &mut PgTransaction,
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

    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

async fn get_newsletter_issue(
    transaction: &mut PgTransaction,
    issue_id: Uuid,
) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
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

    Ok(issue)
}

pub async fn cleanup_old_idempotency_records(pool: &PgPool) -> Result<(), anyhow::Error> {
    let deleted =
        sqlx::query!(r#"DELETE FROM idempotency WHERE created_at < NOW() - INTERVAL '48 hours'"#)
            .execute(pool)
            .await?
            .rows_affected();

    tracing::info!(deleted, "Idempotency cleanup completed");
    Ok(())
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
