use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use moodfeed::configuration::{get_config, DatabaseConfigs};
use moodfeed::startup::run;
use moodfeed::telemetry;
use std::sync::LazyLock;
use secrecy::Secret;
use moodfeed::email_client::EmailClient;
use reqwest::Url;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

// Ensure that the `tracing` stack is only initialised once using `LazyLock`
static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    // If TEST_LOG env variable is set then output the logs to std out while running tests. Otherwise skip
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = telemetry::get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stdout
        );
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber = telemetry::get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::sink
        );
        telemetry::init_subscriber(subscriber);
    };
});


async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    LazyLock::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_config().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let sender_email = configuration.email_client.sender()
        .expect("Failed to get sender email");

    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        Url::parse(&configuration.email_client.base_url)
            .expect("Invalid email client base URL"),
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );

    let server = run(listener, connection_pool.clone(), email_client).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseConfigs) -> PgPool {
    // Create database
    let maintenance_settings = DatabaseConfigs {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: Secret::new("password".to_string()),
        ..config.clone()
    };
    let mut connection = PgConnection::connect_with(
        &maintenance_settings.connect_options()
    )
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

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn add_user_returns_a_200_for_valid_json_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "name": "athfantest",
        "email": "athfantest@gmail.com"
    });

    let response = client
        .post(format!("{}/user/add", app.address))
        .header("Content-Type", "application/json")
        .json(&payload) // Automatically sets body and content-type
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());

    let saved = sqlx::query!("SELECT email, name FROM users",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved user data.");

    assert_eq!(saved.email, "athfantest@gmail.com");
    assert_eq!(saved.name, "athfantest");
}

#[tokio::test]
async fn add_user_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        (serde_json::json!({ "name": "athfan" }), "missing the email"),
        (serde_json::json!({ "email": "athfantest@gmail.com" }), "missing the name"),
        (serde_json::json!({}), "missing both name and email"),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = client
            .post(format!("{}/user/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&invalid_payload)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}."
        );
    }
}

#[tokio::test]
async fn add_user_returns_a_400_when_data_is_present_but_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        (serde_json::json!({ "name": "athfan", "email": "" }), "empty email string"),
        (serde_json::json!({ "email": "athfantest@gmail.com", "name": "" }), "empty name string"),
        (serde_json::json!({"name": "athfan", "email": "definitely wrong email"}), "invalid email address"),
        (serde_json::json!({"name": "ath/fan)", "email": "athfantest@gmail.com"}), "name contains invalid characters"),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = client
            .post(format!("{}/user/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&invalid_payload)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {_error_message}."
        );
    }
}

