use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use moodfeed::configuration::{get_config, DatabaseConfigs};
use moodfeed::startup::get_connection_pool;
use moodfeed::telemetry;
use std::sync::LazyLock;
use secrecy::Secret;
use moodfeed::startup::Application;

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


pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    LazyLock::force(&TRACING);

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_config().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // Use a random OS port
        c.application.port = 0;
        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    // Launch the application as a background task
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address: format!("http://localhost:{}", application_port),
        db_pool: get_connection_pool(&configuration.database),
    }
}

async fn configure_database(config: &DatabaseConfigs) -> PgPool {
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
