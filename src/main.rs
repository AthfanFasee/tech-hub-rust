use moodfeed::configuration;
use moodfeed::startup::Application;
use moodfeed::telemetry;

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("Server startup error: {e}");
    }
}

async fn try_main() -> Result<(), std::io::Error> {
    let subscriber = telemetry::get_subscriber("moodfeed".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);
    let config = configuration::get_config().expect("Failed to read config");
    let application = Application::build(config).await?;
    application.run_until_stopped().await?;
    Ok(())
}
