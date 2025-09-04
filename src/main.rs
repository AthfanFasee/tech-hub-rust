use std::net::TcpListener;
use sqlx::postgres::PgPoolOptions;
use moodfeed::startup::run;
use moodfeed::configuration;
use moodfeed::telemetry;
use moodfeed::email_client::EmailClient;
use reqwest::Url;

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

    let connection_pool = PgPoolOptions::new()
        .connect_lazy_with(config.database.connect_options());
    
    let sender_email = config.email_client.sender()
        .expect("Failed to get sender email");

    let timeout = config.email_client.timeout();
    let email_client = EmailClient::new(
        Url::parse(&config.email_client.base_url)
            .expect("Invalid email client base URL"),
        sender_email,
        config.email_client.authorization_token,
        timeout,
    );


    let address = format!(
        "{}:{}",
        config.application.host, config.application.port
    );
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool,email_client)?.await?;
    Ok(())
}