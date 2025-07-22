use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UserData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new user",
    skip(pool, payload),
    fields(
        user_email = %payload.email,
        user_name = %payload.name,
    )
)]
pub async fn add_user(payload: web::Json<UserData>, pool: web::Data<PgPool>) -> HttpResponse {
  match insert_user(&payload, &pool).await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[tracing::instrument(
name = "Saving new user details in the database",
skip(payload, pool)
)]
pub async fn insert_user(
    payload: &UserData,
    pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO users (id, name, email)
	    VALUES ($1, $2, $3)
	   "#,
        Uuid::new_v4(),
        payload.name,
        payload.email,
    ).execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;

    Ok(())
}

