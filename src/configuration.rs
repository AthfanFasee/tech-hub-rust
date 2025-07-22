use serde;
use secrecy::{ExposeSecret, Secret};

#[derive(serde::Deserialize)]
pub struct Configuration {
    pub application_port: u16,
    pub database: DatabaseConfigs,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseConfigs {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

pub fn get_config() -> Result<Configuration, config::ConfigError> {
    // initialise config reader
    let configs = config::Config::builder()
        .add_source(
            config::File::new("config.yaml", config::FileFormat::Yaml)
        ).build()?;

    // convert the config values to config type
    configs.try_deserialize::<Configuration>()
}

impl DatabaseConfigs {
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password.expose_secret(), self.host, self.port, self.database_name
        ))
    }
}