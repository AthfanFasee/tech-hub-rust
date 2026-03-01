use crate::helpers;

#[tokio::test]
async fn cleanup_old_idempotency_records_deletes_records_older_than_48_hours() {
    let app = helpers::spawn_app().await;
    let pool = &app.db_pool;

    // Insert one old record (older than 48h)
    sqlx::query!(
        r#"
        INSERT INTO idempotency (user_id, idempotency_key, created_at)
        VALUES ($1, $2, NOW() - INTERVAL '50 hours')
        "#,
        app.test_user.user_id,
        "old-key"
    )
    .execute(pool)
    .await
    .unwrap();

    // Insert one recent record (newer than 24h)
    sqlx::query!(
        r#"
        INSERT INTO idempotency (user_id, idempotency_key, created_at)
        VALUES ($1, $2, NOW() - INTERVAL '2 hours')
        "#,
        app.test_user.user_id,
        "new-key"
    )
    .execute(pool)
    .await
    .unwrap();

    app.cleanup_old_idempotency_records().await;

    // The old record should be deleted
    let old_exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM idempotency WHERE idempotency_key = $1)"#,
        "old-key"
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap();
    assert!(!old_exists, "Old record was not deleted");

    // The recent record should still exist
    let new_exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM idempotency WHERE idempotency_key = $1)"#,
        "new-key"
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap();
    assert!(new_exists, "Recent record was wrongly deleted");
}
