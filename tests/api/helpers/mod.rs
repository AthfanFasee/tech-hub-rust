mod admin;
mod comment;
mod http;
mod post;
mod user;

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use secrecy::Secret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::OnceLock;
use techhub::configuration::{DatabaseConfigs};
use techhub::configuration;
use techhub::email_client::EmailClient;
use techhub::startup::{Application};
use techhub::startup;
use techhub::telemetry;
use uuid::Uuid;
use wiremock::MockServer;

#[derive(Debug)]
pub struct TestUser {
    pub user_id: Uuid,
    pub user_name: String,
    pub password: String,
    pub email: String,
}

impl TestUser {
    pub fn generate() -> Self {
        let random_uuid = Uuid::new_v4().to_string();
        Self {
            user_id: Uuid::new_v4(),
            user_name: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
            email: format!("{}@gmail.com", random_uuid),
        }
    }

    pub async fn store(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let test_params = Params::new(100, 1, 1, None).unwrap();
        let password_hash = Argon2::new(Algorithm::Argon2id, Version::V0x13, test_params)
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            r#"
            INSERT INTO users (id, user_name, password_hash, email, is_activated)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            self.user_id,
            self.user_name,
            password_hash,
            self.email,
            true,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

static TRACING: OnceLock<()> = OnceLock::new();

pub fn init_tracing() {
    TRACING.get_or_init(|| {
        let default_filter_level = "info".to_string();
        let subscriber_name = "test".to_string();

        if std::env::var("TEST_LOG").is_ok() {
            let subscriber = telemetry::get_subscriber(
                subscriber_name.clone(),
                default_filter_level.clone(),
                std::io::stdout,
            );
            telemetry::init_subscriber(subscriber);
        } else {
            let subscriber = telemetry::get_subscriber(
                subscriber_name.clone(),
                default_filter_level.clone(),
                std::io::sink,
            );
            telemetry::init_subscriber(subscriber);
        };
    });
}

pub async fn spawn_app() -> TestApp {
    init_tracing();

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = configuration::get_config().expect("Failed to read configuration.");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool: startup::get_connection_pool(&configuration.database),
        email_server,
        test_user: TestUser::generate(),
        api_client: client,
        email_client: configuration.email_client.client(),
    };

    test_app
        .test_user
        .store(&test_app.db_pool)
        .await
        .expect("Failed to store test user");

    test_app
}

async fn configure_database(config: &DatabaseConfigs) -> PgPool {
    let maintenance_settings = DatabaseConfigs {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: Secret::new("password".to_string()),
        ..config.clone()
    };

    let mut connection = PgConnection::connect_with(&maintenance_settings.connect_options())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let connection_pool = PgPool::connect_with(config.connect_options())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
