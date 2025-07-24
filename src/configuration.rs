use serde;
use secrecy::{ExposeSecret, Secret};
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgSslMode;


#[derive(serde::Deserialize)]
pub struct Configuration {
    pub application: ApplicationSettings,
    pub database: DatabaseConfigs,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseConfigs {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

pub fn get_config() -> Result<Configuration, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to get current directory path");
    let config_directory = base_path.join("configuration");

    // Detect running environment
    // Default to local if unspecified
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    let environment_filename = format!("{}.yaml", environment.as_str());
    // initialise config reader
    let configs = config::Config::builder()
        .add_source(
            config::File::from(config_directory.join("base.yaml"))
        )
        .add_source(
            config::File::from(config_directory.join(environment_filename))
        )
        .build()?;

    // convert the config values to config type
    configs.try_deserialize::<Configuration>()
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!("Unknown environment: {other}. Use either `local` or `production`")),
        }
    }
}

impl DatabaseConfigs {
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // Try an encrypted connection, fallback to unencrypted if it fails
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name)
    }

}