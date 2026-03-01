use anyhow::Context;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[tracing::instrument(skip(token, pool))]
pub async fn store_subscription_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"INSERT INTO tokens (token, user_id, is_subscription)
            VALUES ($1, $2, $3)"#,
        token,
        user_id,
        true,
    )
    .execute(pool)
    .await
    .context("Failed to store the user subscription token")?;

    Ok(())
}

#[tracing::instrument(skip(token, transaction))]
pub async fn store_activation_token(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    token: &str,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO tokens (token, user_id, is_activation)
            VALUES ($1, $2, $3)"#,
        token,
        user_id,
        true,
    );

    transaction
        .execute(query)
        .await
        .context("Failed to store the user activation token")?;
    Ok(())
}

pub async fn get_user_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, anyhow::Error> {
    let result = sqlx::query!(
        "SELECT user_id FROM tokens \
            WHERE token = $1",
        token,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to retrieve the user id associated with the provided token.")?;
    Ok(result.map(|r| r.user_id))
}
