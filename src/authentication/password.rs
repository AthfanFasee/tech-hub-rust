use anyhow::Context;
use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{repository, telemetry};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub user_name: String,
    pub password: Secret<String>,
}

#[tracing::instrument(skip_all)]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, AuthError> {
    let mut user_id = None;

    // Dummy hash ensures constant-time response even for unknown usernames, timing-based or user enumeration vulnerability attacks
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        repository::get_stored_credentials(&credentials.user_name, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    // `expected_password_hash` and `credentials.password` are moved into the closure
    //  f - the closure (which spawn_blocking_with_tracing receives) now owns these values fully.
    telemetry::spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    // Always verify hash before checking user_id to prevent timing-based or user enumeration vulnerability attacks
    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username."))
        .map_err(AuthError::InvalidCredentials)
}

fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(skip(password, pool))]
pub async fn change_password(
    user_id: Uuid,
    password: Secret<String>,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash =
        telemetry::spawn_blocking_with_tracing(move || compute_password_hash(password))
            .await?
            .context("Failed to hash password")?;

    repository::update_password_hash(user_id, password_hash, pool).await
}
pub fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        // Safe to panic here as params are hardcoded constants, any failure would be caught at dev/test time
        Params::new(15000, 2, 1, None).expect("Hardcoded Argon2 parameters should always be valid"),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();
    Ok(Secret::new(password_hash))
}
