use techhub::configuration;
use techhub::startup::Application;
use techhub::telemetry;

#[derive(thiserror::Error, Debug)]
pub enum StartupError {
    #[error(transparent)]
    Startup(#[from] anyhow::Error),
}

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("Server startup error: {e}");
    }
}

async fn try_main() -> Result<(), StartupError> {
    let subscriber = telemetry::get_subscriber("techhub".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);
    let config = configuration::get_config().expect("Failed to read config");
    let application = Application::build(config).await?;
    application.run_until_stopped().await?;
    Ok(())
}
