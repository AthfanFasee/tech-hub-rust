use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use secrecy::Secret;
use serde_json::Value;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::OnceLock;
use techhub::configuration::{DatabaseConfigs, get_config};
use techhub::startup::{Application, get_connection_pool};
use techhub::telemetry;
use uuid::Uuid;
use wiremock::MockServer;

#[derive(Debug)]
pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
    pub email: String,
}

impl TestUser {
    pub fn generate() -> Self {
        let random_uuid = Uuid::new_v4().to_string();
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
            email: format!("{}@gmail.com", random_uuid),
        }
    }

    pub async fn store(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        let salt = SaltString::generate(&mut rand::thread_rng());

        // Use lighter parameters for test performance
        let test_params = Params::new(100, 1, 1, None).unwrap();
        let password_hash = Argon2::new(Algorithm::Argon2id, Version::V0x13, test_params)
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            r#"INSERT INTO users (id, name, password_hash, email, is_activated)
            VALUES ($1, $2, $3, $4, $5)"#,
            self.user_id,
            self.username,
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
}

// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    /// Extract the confirmation links embedded in the request to the email API.
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

            // Make sure correct API is called
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn register_user(&self, payload: &Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/user/register", &self.address))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(payload)
            .send()
            .await
            .expect("Failed to execute request: add_user")
    }

    pub async fn publish_newsletters(&self, payload: &Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/newsletters/publish", &self.address))
            .json(payload)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn login(&self) {
        let login_body = serde_json::json!({
        "username": &self.test_user.username,
        "password": &self.test_user.password
        });

        let response = self
            .api_client
            .post(&format!("{}/user/login", &self.address))
            .json(&login_body)
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(response.status().as_u16(), 200);
    }

    pub async fn login_custom_credentials(&self, payload: &Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/user/login", &self.address))
            .json(payload)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn login_admin(&self) {
        let login_body = serde_json::json!({
            "username": "athfan",
            "password": "athfan123"
        });

        let response = self
            .api_client
            .post(&format!("{}/user/login", &self.address))
            .json(&login_body)
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(response.status().as_u16(), 200);
    }

    pub async fn change_password(&self, payload: &Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/user/reset-password", &self.address))
            .json(&payload)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn logout(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/user/logout", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn access_protected_endpoint(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/user/protected", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn send_subscribe_email(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/user/email/subscribe", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

// Ensure that the `tracing` stack is only initialised once using `OnceLock`
static TRACING: OnceLock<()> = OnceLock::new();

pub fn init_tracing() {
    TRACING.get_or_init(|| {
        let default_filter_level = "info".to_string();
        let subscriber_name = "test".to_string();

        // If TEST_LOG env variable is set then output the logs to stdout while running tests
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
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    init_tracing();

    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_config().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // Use a random OS port
        c.application.port = 0;
        // Use the mock server as email API
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    // Launch the application as a background task
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
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        test_user: TestUser::generate(),
        api_client: client,
    };

    test_app
        .test_user
        .store(&test_app.db_pool)
        .await
        .expect("Failed to store test user");

    test_app
}

async fn configure_database(config: &DatabaseConfigs) -> PgPool {
    // Create database
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

    // Migrate database
    let connection_pool = PgPool::connect_with(config.connect_options())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
