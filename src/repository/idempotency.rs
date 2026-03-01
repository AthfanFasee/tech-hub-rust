use sqlx::PgPool;

pub async fn cleanup_old_idempotency_records(pool: &PgPool) -> Result<(), anyhow::Error> {
    let deleted =
        sqlx::query!(r#"DELETE FROM idempotency WHERE created_at < NOW() - INTERVAL '48 hours'"#)
            .execute(pool)
            .await?
            .rows_affected();

    tracing::info!(deleted, "Idempotency cleanup completed");
    Ok(())
}
