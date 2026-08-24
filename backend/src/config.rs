use std::{env, net::IpAddr};

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct Config {
    pub environment: Environment,
    pub host: IpAddr,
    pub port: u16,
    pub database_url: Option<String>,
    pub trust_proxy_headers: bool,
    pub web_origins: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Environment {
    Development,
    Test,
    Staging,
    Production,
}

impl Environment {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::parse(env::var("APP_ENV").ok().as_deref())
    }

    pub const fn is_deployed(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }

    fn parse(value: Option<&str>) -> Result<Self, ConfigError> {
        match value.unwrap_or("development") {
            "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            _ => Err(ConfigError::InvalidEnvironment),
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ConfigError {
    #[error("APP_ENV must be one of: development, test, staging, production")]
    InvalidEnvironment,
    #[error("HOST must be a valid IP address")]
    InvalidHost,
    #[error("PORT must be a valid TCP port")]
    InvalidPort,
    #[error("DATABASE_URL is required in staging and production")]
    MissingDeploymentDatabase,
    #[error("TRUST_PROXY_HEADERS must be true or false")]
    InvalidTrustProxyHeaders,
    #[error(
        "WEB_ORIGINS must be a comma-separated list of absolute HTTP(S) origins without paths; production origins must use HTTPS"
    )]
    InvalidWebOrigins,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_values(
            env::var("APP_ENV").ok().as_deref(),
            env::var("HOST").ok().as_deref(),
            env::var("PORT").ok().as_deref(),
            env::var("DATABASE_URL").ok(),
            env::var("TRUST_PROXY_HEADERS").ok().as_deref(),
            env::var("WEB_ORIGINS").ok().as_deref(),
        )
    }

    fn from_values(
        environment: Option<&str>,
        host: Option<&str>,
        port: Option<&str>,
        database_url: Option<String>,
        trust_proxy_headers: Option<&str>,
        web_origins: Option<&str>,
    ) -> Result<Self, ConfigError> {
        let environment = Environment::parse(environment)?;
        let host = host
            .unwrap_or("0.0.0.0")
            .parse()
            .map_err(|_| ConfigError::InvalidHost)?;
        let port = port
            .unwrap_or("8080")
            .parse()
            .map_err(|_| ConfigError::InvalidPort)?;

        if environment.is_deployed() && database_url.as_deref().is_none_or(str::is_empty) {
            return Err(ConfigError::MissingDeploymentDatabase);
        }
        let trust_proxy_headers = match trust_proxy_headers.unwrap_or("false") {
            "true" => true,
            "false" => false,
            _ => return Err(ConfigError::InvalidTrustProxyHeaders),
        };
        let web_origins = parse_web_origins(environment, web_origins)?;

        Ok(Self {
            environment,
            host,
            port,
            database_url,
            trust_proxy_headers,
            web_origins,
        })
    }
}

fn parse_web_origins(
    environment: Environment,
    value: Option<&str>,
) -> Result<Vec<String>, ConfigError> {
    let default =
        "http://127.0.0.1:3000,http://localhost:3000,http://127.0.0.1:3001,http://localhost:3001";
    let raw = value.unwrap_or(if environment.is_deployed() {
        ""
    } else {
        default
    });
    let origins = raw
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            let uri = origin
                .parse::<axum::http::Uri>()
                .map_err(|_| ConfigError::InvalidWebOrigins)?;
            let valid_scheme = matches!(uri.scheme_str(), Some("http" | "https"));
            let deployment_https = !environment.is_deployed() || uri.scheme_str() == Some("https");
            if !valid_scheme
                || !deployment_https
                || uri.authority().is_none()
                || uri.path() != "/"
                || uri.query().is_some()
                || origin.contains('@')
            {
                return Err(ConfigError::InvalidWebOrigins);
            }
            Ok(origin.trim_end_matches('/').to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if origins.is_empty() {
        return Err(ConfigError::InvalidWebOrigins);
    }
    Ok(origins)
}

#[cfg(test)]
mod tests {
    use super::{Config, ConfigError, Environment};

    #[test]
    fn development_defaults_are_safe_and_predictable() {
        let config = Config::from_values(None, None, None, None, None, None).unwrap();
        assert_eq!(config.environment, Environment::Development);
        assert_eq!(config.host.to_string(), "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(!config.trust_proxy_headers);
    }

    #[test]
    fn production_requires_a_database() {
        let error =
            Config::from_values(Some("production"), None, None, None, None, None).unwrap_err();
        assert_eq!(error, ConfigError::MissingDeploymentDatabase);
    }

    #[test]
    fn staging_requires_a_database() {
        let error = Config::from_values(Some("staging"), None, None, None, None, None).unwrap_err();
        assert_eq!(error, ConfigError::MissingDeploymentDatabase);
    }

    #[test]
    fn invalid_ports_fail_early() {
        let error = Config::from_values(None, None, Some("70000"), None, None, None).unwrap_err();
        assert_eq!(error, ConfigError::InvalidPort);
    }

    #[test]
    fn proxy_header_trust_is_explicit() {
        let config = Config::from_values(None, None, None, None, Some("true"), None).unwrap();
        assert!(config.trust_proxy_headers);
        let error = Config::from_values(None, None, None, None, Some("yes"), None).unwrap_err();
        assert_eq!(error, ConfigError::InvalidTrustProxyHeaders);
    }

    #[test]
    fn production_requires_explicit_https_web_origins() {
        let missing = Config::from_values(
            Some("production"),
            None,
            None,
            Some("postgres://example".into()),
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(missing, ConfigError::InvalidWebOrigins);

        let insecure = Config::from_values(
            Some("production"),
            None,
            None,
            Some("postgres://example".into()),
            None,
            Some("http://shop.example.com"),
        )
        .unwrap_err();
        assert_eq!(insecure, ConfigError::InvalidWebOrigins);

        let config = Config::from_values(
            Some("production"),
            None,
            None,
            Some("postgres://example".into()),
            None,
            Some("https://shop.example.com,https://admin.example.com"),
        )
        .unwrap();
        assert_eq!(config.web_origins.len(), 2);
    }

    #[test]
    fn staging_requires_explicit_https_web_origins() {
        let missing = Config::from_values(
            Some("staging"),
            None,
            None,
            Some("postgres://example".into()),
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(missing, ConfigError::InvalidWebOrigins);

        let insecure = Config::from_values(
            Some("staging"),
            None,
            None,
            Some("postgres://example".into()),
            None,
            Some("http://shop.example.com"),
        )
        .unwrap_err();
        assert_eq!(insecure, ConfigError::InvalidWebOrigins);

        let config = Config::from_values(
            Some("staging"),
            None,
            None,
            Some("postgres://example".into()),
            None,
            Some("https://shop.example.com,https://admin.example.com"),
        )
        .unwrap();
        assert_eq!(config.environment, Environment::Staging);
        assert_eq!(config.web_origins.len(), 2);
    }
}
