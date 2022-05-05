#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub database_name: String,
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
}

impl DatabaseSettings {
    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port,
        )
    }

    pub fn connection_string_for_db(&self, database_name: &str) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, database_name,
        )
    }

    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name,
        )
    }
}

#[derive(serde::Deserialize)]
pub struct Configuration {
    pub database: DatabaseSettings,
    pub host: String,
    pub port: u16,
}

pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    // initialize our configuration reader
    let mut settings = config::Config::default();

    // Add configuration values from a file named `config`.
    // It will look for any top-level file with an extension
    // that `config` knows how to handled/parser: yaml, json, etc

    settings.merge(config::File::with_name("config"))?;

    // try converting settings into `Configuration` object.
    return settings.try_into();
}
