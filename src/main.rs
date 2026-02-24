use std::fmt::{Debug, Display};
use techhub::configuration;
use techhub::newsletter_delivery_worker::run_worker_until_stopped;
use techhub::startup;
use techhub::telemetry;
use tokio::task::JoinError;

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("Server startup error: {e}");
    }
}

async fn try_main() -> anyhow::Result<()> {
    let subscriber = telemetry::get_subscriber("techhub".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);
    let config = configuration::get_config().expect("Failed to read config");
    let application = startup::Application::build(config.clone()).await?;

    let application_task = tokio::spawn(application.run_until_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(config));

    tokio::select! {
        o = application_task => {
            report_exit("API", &o);
            o??
        },
        o = worker_task => {
            report_exit("Newsletter issue background worker", &o);
            o??
        },
    }

    Ok(())
}

fn report_exit(task_name: &str, outcome: &Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} task failed to complete",
                task_name
            )
        }
    }
}
