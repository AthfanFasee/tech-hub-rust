use std::net::TcpListener;
use sqlx::PgPool;
use moodfeed::startup::run;
use moodfeed::configuration;

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("Server startup error: {e}");
    }
}

async fn try_main() -> Result<(), std::io::Error> {
    let config = configuration::get_config().expect("Failed to read config");

    let connection_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to Postgres database");

    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}