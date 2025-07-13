use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use moodfeed::configuration::{get_config, DatabaseConfigs};
use moodfeed::startup::run;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_config().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");
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
        password: "password".to_string(),
        ..config.clone()
    };
    let mut connection = PgConnection::connect(&maintenance_settings.connection_string())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string())
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
async fn subscribe_returns_a_200_for_valid_json_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "name": "athfantest",
        "email": "athfantest@gmail.com"
    });

    let response = client
        .post(format!("{}/subscriptions", app.address))
        .header("Content-Type", "application/json")
        .json(&payload) // Automatically sets body and content-type
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());

    let saved = sqlx::query!("SELECT email, name FROM users",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "athfantest@gmail.com");
    assert_eq!(saved.name, "athfantest");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        (serde_json::json!({ "name": "athfan" }), "missing the email"),
        (serde_json::json!({ "email": "athfantest@gmail.com" }), "missing the name"),
        (serde_json::json!({}), "missing both name and email"),
    ];

    for (invalid_payload, _error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &app.address))
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

