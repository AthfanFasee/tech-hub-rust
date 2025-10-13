use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use reqwest::Response;
use reqwest::header::HeaderMap;
use secrecy::Secret;
use serde_json::Value;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::OnceLock;
use techhub::configuration::{DatabaseConfigs, get_config};
use techhub::email_client::EmailClient;
use techhub::newsletter_delivery_worker::{ExecutionOutcome, try_execute_task};
use techhub::startup::{Application, get_connection_pool};
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

        // Use lighter parameters for test performance
        let test_params = Params::new(100, 1, 1, None).unwrap();
        let password_hash = Argon2::new(Algorithm::Argon2id, Version::V0x13, test_params)
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            r#"INSERT INTO users (id, user_name, password_hash, email, is_activated)
            VALUES ($1, $2, $3, $4, $5)"#,
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

// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    async fn send_get(&self, endpoint: &str) -> Response {
        self.api_client
            .get(format!("{}/{}", &self.address, endpoint))
            .send()
            .await
            .expect("Failed to execute GET request.")
    }

    async fn send_post(&self, endpoint: &str, payload: &Value) -> Response {
        self.api_client
            .post(format!("{}/{}", &self.address, endpoint))
            .json(payload)
            .send()
            .await
            .expect("Failed to execute POST request.")
    }

    pub async fn send_post_with_headers(
        &self,
        endpoint: &str,
        payload: &Value,
        headers: &HeaderMap,
    ) -> Response {
        self.api_client
            .post(format!("{}/{}", &self.address, endpoint))
            .json(payload)
            .headers(headers.clone())
            .send()
            .await
            .expect("Failed to execute POST request.")
    }

    pub async fn register_user(&self, payload: &Value) -> Response {
        self.send_post("v1/user/register", payload).await
    }

    pub async fn login(&self) {
        let body = serde_json::json!({
            "user_name": &self.test_user.user_name,
            "password": &self.test_user.password,
        });
        let response = self.send_post("v1/user/login", &body).await;
        assert_eq!(response.status().as_u16(), 200);
    }

    pub async fn login_with(&self, creds: &Value) -> Response {
        self.send_post("v1/user/login", creds).await
    }

    pub async fn login_admin(&self) {
        let body = serde_json::json!({
            "user_name": "athfan",
            "password": "athfan123",
        });

        let response = self.send_post("v1/user/login", &body).await;
        assert_eq!(response.status().as_u16(), 200);
    }

    pub async fn change_password(&self, payload: &Value) -> Response {
        self.send_post("v1/user/me/reset-password", payload).await
    }

    pub async fn logout(&self) -> Response {
        self.send_post("v1/user/me/logout", &serde_json::json!({}))
            .await
    }

    pub async fn access_protected(&self) -> Response {
        self.send_get("v1/user/me/protected").await
    }

    pub async fn publish_newsletters(
        &self,
        payload: &Value,
        idempotency_key: Option<&String>,
    ) -> Response {
        if let Some(key) = idempotency_key {
            let mut headers = HeaderMap::new();
            headers.insert("Idempotency-Key", key.parse().unwrap());
            self.send_post_with_headers("v1/admin/me/newsletters/publish", payload, &headers)
                .await
        } else {
            self.send_post("v1/admin/me/newsletters/publish", payload)
                .await
        }
    }

    pub async fn send_subscribe_email(&self) -> Response {
        self.send_get("v1/user/me/email/subscribe").await
    }

    pub async fn dispatch_all_pending_newsletter_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
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
