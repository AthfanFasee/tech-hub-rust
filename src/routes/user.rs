use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use crate::domain::{NewUser, UserName, UserEmail};

#[derive(Deserialize)]
pub struct UserData {
    email: String,
    name: String,
}

// This is like saying - I know how to build myself (NewUser) from something else (UserData)
// Then Rust lets us use `.try_into` whenever there's a UserData (where it automatically tries converting it to a NewUser)
impl TryFrom<UserData> for NewUser {
    type Error = String;

    fn try_from(payload: UserData) -> Result<Self, Self::Error> {
        let name = UserName::parse(payload.name)?;
        let email = UserEmail::parse(payload.email)?;
        Ok(Self { name, email })
    }
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
    let new_user = match payload.0.try_into() {
        Ok(payload) => payload,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match insert_user(&new_user, &pool).await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[tracing::instrument(
    name = "Saving new user details in the database",
    skip(new_user, pool)
)]
pub async fn insert_user(
    new_user: &NewUser,
    pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO users (id, name, email)
	    VALUES ($1, $2, $3)
	   "#,
        Uuid::new_v4(),
        new_user.name.as_ref(),
        new_user.email.as_ref(),
    ).execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;

    Ok(())
}

