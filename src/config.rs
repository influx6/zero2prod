use env_logger::Env;
use secrecy::{ExposeSecret, Secret};

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

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other,
            )),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub database_name: String,
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
}

impl DatabaseSettings {
    pub fn connection_string_without_db(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
        ))
    }

    pub fn connection_string_for_db(&self, database_name: &str) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            database_name,
        ))
    }

    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name,
        ))
    }
}

#[derive(serde::Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
}

#[derive(serde::Deserialize)]
pub struct Configuration {
    pub database: DatabaseSettings,
    pub app: AppConfig,
}

pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    // initialize our configuration reader
    let mut settings = config::Config::default();

    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");

    // Add configuration values from a file named `config`.
    // It will look for any top-level file with an extension
    // that `config` knows how to handled/parser: yaml, json, etc

    // Read in default configuration
    settings.merge(config::File::from(configuration_directory.join("base")).required(true))?;

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    // Read in layer environment specific file.
    settings.merge(
        config::File::from(configuration_directory.join(environment.as_str())).required(true),
    )?;

    // try converting settings into `Configuration` object.
    return settings.try_into();
}
