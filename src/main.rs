use std::net::TcpListener;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use moodfeed::startup::run;
use moodfeed::configuration;
use moodfeed::telemetry;

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("Server startup error: {e}");
    }
}

async fn try_main() -> Result<(), std::io::Error> {
    let subscriber = telemetry::get_subscriber("moodfeed".into(), "info".into(), std::io::stdout
    );
    telemetry::init_subscriber(subscriber);
    
    let config = configuration::get_config().expect("Failed to read config");

    let connection_pool = PgPool::connect(config.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres database");

    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}