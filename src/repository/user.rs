use anyhow::Context;
use secrecy::{ExposeSecret, Secret};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::{UserEmail, UserName};

#[tracing::instrument(skip_all)]
pub async fn insert_user(
    user_name: &UserName,
    email: &UserEmail,
    password_hash: Secret<String>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, anyhow::Error> {
    let user_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO users (id, user_name, email, password_hash)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        user_name.as_ref(),
        email.as_ref(),
        password_hash.expose_secret()
    );

    transaction
        .execute(query)
        .await
        .context("Failed to insert new user")?;
    Ok(user_id)
}

#[tracing::instrument(skip(pool, token))]
pub async fn activate_user_and_delete_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        WITH activate_user AS (
            UPDATE users
            SET is_activated = true
            WHERE id = $1
        )
        DELETE FROM tokens
        WHERE token = $2 AND user_id = $1 AND is_activation = true
        "#,
        user_id,
        token,
    )
    .execute(pool)
    .await
    .context("Failed to update the user status as activated")?;

    Ok(())
}

#[tracing::instrument(skip(pool, token))]
pub async fn subscribe_user_and_delete_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        WITH subscribe_user AS (
            UPDATE users
            SET is_subscribed = true
            WHERE id = $1 and is_activated = true
        )
        DELETE FROM tokens
        WHERE token = $2 AND user_id = $1 AND is_subscription = true
        "#,
        user_id,
        token,
    )
    .execute(pool)
    .await
    .context("Failed to update the user status as subscribed")?;

    Ok(())
}

pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_name
        FROM users
        WHERE id = $1 and is_activated = true
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.user_name)
}

pub async fn get_user_email(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT email
        FROM users
        WHERE id = $1 and is_activated = true
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a user email.")?;
    Ok(row.email)
}

pub async fn is_admin_user(user_id: Uuid, pool: &PgPool) -> Result<bool, anyhow::Error> {
    let record = sqlx::query!(
        r#"
        SELECT is_admin
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to fetch admin flag for user")?;

    let is_admin = record
        .map(|r| r.is_admin)
        .ok_or_else(|| anyhow::anyhow!("No user found"))?;

    Ok(is_admin)
}

pub async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT id, password_hash
        FROM users
        WHERE user_name = $1
        and is_activated = true
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    // At this point, row is an `Option<Row>`. After this, row becomes `Option<(Uuid, Secret<String>)>`
    .map(|row| (row.id, Secret::new(row.password_hash)));
    Ok(row)
}

pub async fn update_password_hash(
    user_id: Uuid,
    password_hash: Secret<String>,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE id = $2 and is_activated = true
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password")?;
    Ok(())
}
