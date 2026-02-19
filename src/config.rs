use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub app_host: String,
    pub app_port: u16,
    pub app_log_level: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid value for {key}: {message}")]
    InvalidValue { key: String, message: String },
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    /// Load configuration from a custom lookup function.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if required keys are missing or values are invalid.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let database_url = lookup("DATABASE_URL")
            .ok_or_else(|| ConfigError::MissingEnvVar("DATABASE_URL".to_string()))?;
        let database_max_connections = parse_or(&lookup, "DATABASE_MAX_CONNECTIONS", 10)?;
        let database_min_connections = parse_or(&lookup, "DATABASE_MIN_CONNECTIONS", 2)?;
        let app_host = lookup("APP_HOST").unwrap_or_else(|| "0.0.0.0".to_string());
        let app_port = parse_or(&lookup, "APP_PORT", 8080)?;
        let app_log_level =
            lookup("APP_LOG_LEVEL").unwrap_or_else(|| "debug,conf_ops=trace".to_string());

        Ok(Self {
            database_url,
            database_max_connections,
            database_min_connections,
            app_host,
            app_port,
            app_log_level,
        })
    }
}

fn parse_or<T>(
    lookup: &impl Fn(&str) -> Option<String>,
    key: &str,
    default: T,
) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    lookup(key).map_or(Ok(default), |val| {
        val.parse::<T>().map_err(|e| ConfigError::InvalidValue {
            key: key.to_string(),
            message: e.to_string(),
        })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn make_lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    #[test]
    fn missing_database_url_returns_error() {
        let lookup = make_lookup(&[]);
        let result = AppConfig::from_lookup(lookup);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("DATABASE_URL"),
            "Error should mention DATABASE_URL: {err}"
        );
    }

    #[test]
    fn valid_config() {
        let lookup = make_lookup(&[
            ("DATABASE_URL", "postgres://test:test@localhost/test"),
            ("DATABASE_MAX_CONNECTIONS", "20"),
            ("APP_PORT", "3000"),
        ]);

        let config = AppConfig::from_lookup(lookup).expect("should parse valid config");
        assert_eq!(config.database_max_connections, 20);
        assert_eq!(config.app_port, 3000);
        assert_eq!(config.database_min_connections, 2); // default
    }

    #[test]
    fn invalid_port_returns_error() {
        let lookup = make_lookup(&[
            ("DATABASE_URL", "postgres://test:test@localhost/test"),
            ("APP_PORT", "not-a-number"),
        ]);

        let result = AppConfig::from_lookup(lookup);
        assert!(result.is_err());
    }
}
