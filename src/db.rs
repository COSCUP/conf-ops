use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::AppConfig;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Failed to connect to database: {0}")]
    Connection(#[from] sqlx::Error),
}

/// Create a `PostgreSQL` connection pool from the application configuration.
///
/// # Errors
///
/// Returns `DbError::Connection` if the pool cannot be created.
pub async fn create_pool(config: &AppConfig) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .connect(&config.database_url)
        .await?;

    Ok(pool)
}
