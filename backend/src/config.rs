use std::{env, net::IpAddr};

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct Config {
    pub environment: Environment,
    pub host: IpAddr,
    pub port: u16,
    pub database_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ConfigError {
    #[error("APP_ENV must be one of: development, test, production")]
    InvalidEnvironment,
    #[error("HOST must be a valid IP address")]
    InvalidHost,
    #[error("PORT must be a valid TCP port")]
    InvalidPort,
    #[error("DATABASE_URL is required in production")]
    MissingProductionDatabase,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_values(
            env::var("APP_ENV").ok().as_deref(),
            env::var("HOST").ok().as_deref(),
            env::var("PORT").ok().as_deref(),
            env::var("DATABASE_URL").ok(),
        )
    }

    fn from_values(
        environment: Option<&str>,
        host: Option<&str>,
        port: Option<&str>,
        database_url: Option<String>,
    ) -> Result<Self, ConfigError> {
        let environment = match environment.unwrap_or("development") {
            "development" => Environment::Development,
            "test" => Environment::Test,
            "production" => Environment::Production,
            _ => return Err(ConfigError::InvalidEnvironment),
        };
        let host = host
            .unwrap_or("0.0.0.0")
            .parse()
            .map_err(|_| ConfigError::InvalidHost)?;
        let port = port
            .unwrap_or("8080")
            .parse()
            .map_err(|_| ConfigError::InvalidPort)?;

        if environment == Environment::Production
            && database_url.as_deref().is_none_or(str::is_empty)
        {
            return Err(ConfigError::MissingProductionDatabase);
        }

        Ok(Self {
            environment,
            host,
            port,
            database_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, ConfigError, Environment};

    #[test]
    fn development_defaults_are_safe_and_predictable() {
        let config = Config::from_values(None, None, None, None).unwrap();
        assert_eq!(config.environment, Environment::Development);
        assert_eq!(config.host.to_string(), "0.0.0.0");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn production_requires_a_database() {
        let error = Config::from_values(Some("production"), None, None, None).unwrap_err();
        assert_eq!(error, ConfigError::MissingProductionDatabase);
    }

    #[test]
    fn invalid_ports_fail_early() {
        let error = Config::from_values(None, None, Some("70000"), None).unwrap_err();
        assert_eq!(error, ConfigError::InvalidPort);
    }
}
