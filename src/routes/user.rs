use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::{Postgres, Transaction, Executor, PgPool};
use uuid::Uuid;
use crate::domain::{NewUser, UserName, UserEmail};
use crate::email_client::EmailClient;
use crate::email_client::EmailError;
use crate::startup::ApplicationBaseUrl;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use actix_web::ResponseError;

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
    skip(pool, payload, email_client, base_url),
    fields(
        user_email = %payload.email,
        user_name = %payload.name,
    )
)]
pub async fn add_user(
    payload: web::Json<UserData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>
) -> HttpResponse {
    let new_user = match payload.0.try_into() {
        Ok(payload) => payload,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let user_id = match insert_user(&new_user, &mut transaction).await {
        Ok(user_id) => user_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let activation_token = generate_token();

    if store_token(&mut transaction, user_id, &activation_token, true)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if let Err(e) = send_confirmation_email(&email_client, new_user, &base_url.0, &activation_token).await {
        tracing::error!("Failed to send confirmation email: {:?}", e);
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Store token in the database",
    skip(token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    token: &str,
    is_activation: bool,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
            r#"INSERT INTO tokens (token, user_id, is_activation)
            VALUES ($1, $2, $3)"#,
            token,
            user_id,
            is_activation,
        );
    
    transaction.execute(query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}

#[tracing::instrument(
    name = "Save new user details in the database",
    skip(new_user, transaction)
)]
pub async fn insert_user(
    new_user: &NewUser,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let user_id = Uuid::new_v4();
    let query = sqlx::query!(
            r#"
            INSERT INTO users (id, name, email, password_hash)
            VALUES ($1, $2, $3, $4)
           "#,
            user_id,
            new_user.name.as_ref(),
            new_user.email.as_ref(),
            "dummy_hash",
        );
    
    transaction.execute(query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;

    Ok(user_id)
}

#[tracing::instrument(
name = "Send a confirmation email to a new user",
skip(email_client, new_user)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_user: NewUser,
    base_url: &str,
    token: &str,
) -> Result<(), EmailError> {
    let confirmation_link = format!("{base_url}/user/confirm?token={token}");
    let plain_body = format!(
        "Welcome to Moodfeed!\nVisit {confirmation_link} to confirm your subscription.",
    );
    let html_body = format!(
        "Welcome to Moodfeed!<br />\
Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription.",
    );
    email_client
        .send_email(
            new_user.email,
            "Welcome!",
            &html_body,
            &plain_body,
        )
        .await
}

// Generate a random 25-characters-long case-sensitive token.
fn generate_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
