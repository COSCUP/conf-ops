use std::fmt::Write;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::error::AuditError;
use super::models::{AuditLog, AuditLogFilters, CreateAuditLogParams};

fn map_audit_log(row: &sqlx::postgres::PgRow) -> Result<AuditLog, sqlx::Error> {
    let ip_raw: Option<String> = row
        .try_get::<Option<String>, _>("ip_address_text")
        .ok()
        .flatten();

    Ok(AuditLog {
        id: row.try_get("id")?,
        actor_type: row.try_get("actor_type")?,
        actor_id: row.try_get("actor_id")?,
        action: row.try_get("action")?,
        resource_type: row.try_get("resource_type")?,
        resource_id: row.try_get("resource_id")?,
        context_type: row.try_get("context_type")?,
        context_id: row.try_get("context_id")?,
        details: row.try_get("details")?,
        ip_address: ip_raw,
        user_agent: row.try_get("user_agent")?,
        created_at: row.try_get("created_at")?,
    })
}

pub struct AuditLogRepository;

impl AuditLogRepository {
    /// Create a new audit log entry.
    ///
    /// # Errors
    ///
    /// Returns `AuditError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        params: &CreateAuditLogParams,
    ) -> Result<AuditLog, AuditError> {
        let row = sqlx::query(
            r"INSERT INTO audit_logs
              (id, actor_type, actor_id, action, resource_type, resource_id,
               context_type, context_id, details, ip_address, user_agent)
              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::inet, $11)
              RETURNING *, ip_address::text AS ip_address_text",
        )
        .bind(params.id)
        .bind(params.actor_type.as_str())
        .bind(params.actor_id)
        .bind(&params.action)
        .bind(&params.resource_type)
        .bind(params.resource_id)
        .bind(&params.context_type)
        .bind(params.context_id)
        .bind(&params.details)
        .bind(&params.ip_address)
        .bind(&params.user_agent)
        .fetch_one(pool)
        .await?;

        map_audit_log(&row).map_err(AuditError::Database)
    }

    /// Query audit logs with filters.
    ///
    /// Uses dynamic query building to apply optional filters.
    ///
    /// # Errors
    ///
    /// Returns `AuditError::Database` on database failure.
    pub async fn query(
        pool: &PgPool,
        filters: &AuditLogFilters,
    ) -> Result<Vec<AuditLog>, AuditError> {
        // Build dynamic SQL using string concatenation for flexibility
        let mut sql =
            String::from("SELECT *, ip_address::text AS ip_address_text FROM audit_logs WHERE 1=1");
        let mut param_idx = 1u32;
        let mut bind_values: Vec<BindValue> = Vec::new();

        if let Some(ref actor_type) = filters.actor_type {
            let _ = write!(sql, " AND actor_type = ${param_idx}");
            bind_values.push(BindValue::Str(actor_type.clone()));
            param_idx += 1;
        }

        if let Some(actor_id) = filters.actor_id {
            let _ = write!(sql, " AND actor_id = ${param_idx}");
            bind_values.push(BindValue::Uuid(actor_id));
            param_idx += 1;
        }

        if let Some(ref action) = filters.action {
            let _ = write!(sql, " AND action = ${param_idx}");
            bind_values.push(BindValue::Str(action.clone()));
            param_idx += 1;
        }

        if let Some(ref resource_type) = filters.resource_type {
            let _ = write!(sql, " AND resource_type = ${param_idx}");
            bind_values.push(BindValue::Str(resource_type.clone()));
            param_idx += 1;
        }

        if let Some(resource_id) = filters.resource_id {
            let _ = write!(sql, " AND resource_id = ${param_idx}");
            bind_values.push(BindValue::Uuid(resource_id));
            param_idx += 1;
        }

        if let Some(ref context_type) = filters.context_type {
            let _ = write!(sql, " AND context_type = ${param_idx}");
            bind_values.push(BindValue::Str(context_type.clone()));
            param_idx += 1;
        }

        if let Some(context_id) = filters.context_id {
            let _ = write!(sql, " AND context_id = ${param_idx}");
            bind_values.push(BindValue::Uuid(context_id));
            param_idx += 1;
        }

        if let Some(from) = filters.from {
            let _ = write!(sql, " AND created_at >= ${param_idx}");
            bind_values.push(BindValue::DateTime(from));
            param_idx += 1;
        }

        if let Some(to) = filters.to {
            let _ = write!(sql, " AND created_at <= ${param_idx}");
            bind_values.push(BindValue::DateTime(to));
            param_idx += 1;
        }

        if let Some(cursor) = filters.cursor {
            let _ = write!(sql, " AND id < ${param_idx}");
            bind_values.push(BindValue::Uuid(cursor));
            param_idx += 1;
        }

        sql.push_str(" ORDER BY created_at DESC");
        let _ = write!(sql, " LIMIT ${param_idx}");
        bind_values.push(BindValue::I64(filters.limit));

        // Build and execute the query
        let mut query = sqlx::query(&sql);
        for val in &bind_values {
            query = match val {
                BindValue::Str(s) => query.bind(s),
                BindValue::Uuid(u) => query.bind(u),
                BindValue::DateTime(dt) => query.bind(dt),
                BindValue::I64(i) => query.bind(i),
            };
        }

        let rows = query.fetch_all(pool).await?;

        rows.iter()
            .map(|r| map_audit_log(r).map_err(AuditError::Database))
            .collect()
    }

    /// Create monthly partitions for audit logs.
    ///
    /// # Errors
    ///
    /// Returns `AuditError::Database` on database failure.
    pub async fn create_partition(pool: &PgPool, year: i32, month: u32) -> Result<(), AuditError> {
        let partition_name = format!("audit_logs_{year}_{month:02}");
        let start = format!("{year}-{month:02}-01");

        let (next_year, next_month) = if month == 12 {
            (year + 1, 1u32)
        } else {
            (year, month + 1)
        };
        let end = format!("{next_year}-{next_month:02}-01");

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {partition_name} PARTITION OF audit_logs FOR VALUES FROM ('{start}') TO ('{end}')"
        );

        sqlx::query(&sql).execute(pool).await?;
        Ok(())
    }
}

/// Internal helper enum for dynamic query binding.
enum BindValue {
    Str(String),
    Uuid(Uuid),
    DateTime(chrono::DateTime<chrono::Utc>),
    I64(i64),
}
