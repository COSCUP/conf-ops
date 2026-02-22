use tracing_subscriber::EnvFilter;

/// Log format configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFormat {
    /// JSON format for production.
    Json,
    /// Pretty format for development.
    Pretty,
}

impl LogFormat {
    /// Parse log format from string, defaulting to Pretty.
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => Self::Json,
            _ => Self::Pretty,
        }
    }
}

/// Initialize the logging subscriber with the given format and filter.
///
/// # Panics
///
/// Panics if the subscriber cannot be set.
pub fn init_logging(format: &LogFormat, filter: &str) {
    let env_filter = EnvFilter::new(filter);

    match format {
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_thread_ids(true)
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::fmt().with_env_filter(env_filter).init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_format_from_str() {
        assert_eq!(LogFormat::from_str_or_default("json"), LogFormat::Json);
        assert_eq!(LogFormat::from_str_or_default("JSON"), LogFormat::Json);
        assert_eq!(LogFormat::from_str_or_default("pretty"), LogFormat::Pretty);
        assert_eq!(LogFormat::from_str_or_default("other"), LogFormat::Pretty);
    }
}
