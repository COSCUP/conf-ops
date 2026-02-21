use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub app_host: String,
    pub app_port: u16,
    pub app_log_level: String,
    pub app_base_url: String,

    // JWT
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_access_expiry_secs: i64,
    pub jwt_refresh_expiry_secs: i64,

    // WebAuthn
    pub webauthn_rp_id: String,
    pub webauthn_rp_origin: String,
    pub webauthn_rp_name: String,

    // SMTP
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: String,

    // Frontend URL (for magic link redirect)
    pub frontend_url: String,

    // Authorization cache TTL in seconds
    pub authz_cache_ttl_secs: u64,

    // File storage
    pub storage_base_path: String,
    pub storage_max_image_size: u64,
    pub storage_max_document_size: u64,
    pub storage_max_file_size: u64,
    pub storage_cleanup_grace_period_secs: i64,

    // CRDT WebSocket
    pub crdt_ws_max_connections: usize,
    pub crdt_ws_heartbeat_interval_secs: u64,
    pub crdt_ws_idle_timeout_secs: u64,
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
        let app_base_url =
            lookup("APP_BASE_URL").unwrap_or_else(|| "http://localhost:8080".to_string());

        let jwt_secret = lookup("AUTH_JWT_SECRET")
            .ok_or_else(|| ConfigError::MissingEnvVar("AUTH_JWT_SECRET".to_string()))?;
        let jwt_issuer = lookup("AUTH_JWT_ISSUER").unwrap_or_else(|| "conf-ops".to_string());
        let jwt_access_expiry_secs = parse_or(&lookup, "JWT_ACCESS_EXPIRY", 900)?;
        let jwt_refresh_expiry_secs = parse_or(&lookup, "JWT_REFRESH_EXPIRY", 604_800)?;

        let webauthn_rp_id =
            lookup("AUTH_WEBAUTHN_RP_ID").unwrap_or_else(|| "localhost".to_string());
        let webauthn_rp_origin = lookup("AUTH_WEBAUTHN_RP_ORIGIN")
            .unwrap_or_else(|| "http://localhost:3000".to_string());
        let webauthn_rp_name =
            lookup("AUTH_WEBAUTHN_RP_NAME").unwrap_or_else(|| "Conf-Ops".to_string());

        let smtp_host = lookup("SMTP_HOST").unwrap_or_else(|| "localhost".to_string());
        let smtp_port = parse_or(&lookup, "SMTP_PORT", 1025)?;
        let smtp_username = lookup("SMTP_USERNAME");
        let smtp_password = lookup("SMTP_PASSWORD");
        let smtp_from = lookup("SMTP_FROM").unwrap_or_else(|| "noreply@conf-ops.dev".to_string());

        let frontend_url =
            lookup("FRONTEND_URL").unwrap_or_else(|| "http://localhost:3000".to_string());

        let authz_cache_ttl_secs = parse_or(&lookup, "AUTHZ_CACHE_TTL_SECS", 300)?;

        let storage_base_path =
            lookup("STORAGE_BASE_PATH").unwrap_or_else(|| "./storage".to_string());
        let storage_max_image_size = parse_or(&lookup, "STORAGE_MAX_IMAGE_SIZE", 10 * 1024 * 1024)?;
        let storage_max_document_size =
            parse_or(&lookup, "STORAGE_MAX_DOCUMENT_SIZE", 50 * 1024 * 1024)?;
        let storage_max_file_size = parse_or(&lookup, "STORAGE_MAX_FILE_SIZE", 20 * 1024 * 1024)?;
        let storage_cleanup_grace_period_secs =
            parse_or(&lookup, "STORAGE_CLEANUP_GRACE_PERIOD", 7 * 24 * 3600_i64)?;

        let crdt_ws_max_connections = parse_or(&lookup, "CRDT_WS_MAX_CONNECTIONS_PER_TASK", 50)?;
        let crdt_ws_heartbeat_interval_secs =
            parse_or(&lookup, "CRDT_WS_HEARTBEAT_INTERVAL_SECS", 30)?;
        let crdt_ws_idle_timeout_secs = parse_or(&lookup, "CRDT_WS_IDLE_TIMEOUT_SECS", 300)?;

        Ok(Self {
            database_url,
            database_max_connections,
            database_min_connections,
            app_host,
            app_port,
            app_log_level,
            app_base_url,
            jwt_secret,
            jwt_issuer,
            jwt_access_expiry_secs,
            jwt_refresh_expiry_secs,
            webauthn_rp_id,
            webauthn_rp_origin,
            webauthn_rp_name,
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            smtp_from,
            frontend_url,
            authz_cache_ttl_secs,
            storage_base_path,
            storage_max_image_size,
            storage_max_document_size,
            storage_max_file_size,
            storage_cleanup_grace_period_secs,
            crdt_ws_max_connections,
            crdt_ws_heartbeat_interval_secs,
            crdt_ws_idle_timeout_secs,
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

    fn required_pairs() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DATABASE_URL", "postgres://test:test@localhost/test"),
            ("AUTH_JWT_SECRET", "test-secret-key"),
        ]
    }

    #[test]
    fn missing_database_url_returns_error() {
        let lookup = make_lookup(&[("AUTH_JWT_SECRET", "secret")]);
        let result = AppConfig::from_lookup(lookup);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("DATABASE_URL"),
            "Error should mention DATABASE_URL: {err}"
        );
    }

    #[test]
    fn missing_jwt_secret_returns_error() {
        let lookup = make_lookup(&[("DATABASE_URL", "postgres://test:test@localhost/test")]);
        let result = AppConfig::from_lookup(lookup);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("AUTH_JWT_SECRET"),
            "Error should mention AUTH_JWT_SECRET: {err}"
        );
    }

    #[test]
    fn valid_config() {
        let mut pairs = required_pairs();
        pairs.push(("DATABASE_MAX_CONNECTIONS", "20"));
        pairs.push(("APP_PORT", "3000"));
        let lookup = make_lookup(&pairs);

        let config = AppConfig::from_lookup(lookup).expect("should parse valid config");
        assert_eq!(config.database_max_connections, 20);
        assert_eq!(config.app_port, 3000);
        assert_eq!(config.database_min_connections, 2);
        assert_eq!(config.jwt_access_expiry_secs, 900);
        assert_eq!(config.jwt_refresh_expiry_secs, 604_800);
    }

    #[test]
    fn invalid_port_returns_error() {
        let mut pairs = required_pairs();
        pairs.push(("APP_PORT", "not-a-number"));
        let lookup = make_lookup(&pairs);

        let result = AppConfig::from_lookup(lookup);
        assert!(result.is_err());
    }
}
