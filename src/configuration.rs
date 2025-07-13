use serde::Deserialize;

#[derive(Deserialize)]
pub struct Configuration {
    pub application_port: u16,
    pub database: DatabaseConfigs,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfigs {
    pub username: String,
    pub password: String,
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
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
}