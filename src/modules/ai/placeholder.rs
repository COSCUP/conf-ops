use regex::Regex;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// The result of resolving placeholders in a text string.
pub struct ResolveResult {
    /// The text with all resolvable placeholders replaced by their actual values.
    pub text: String,
    /// Placeholder patterns that could not be resolved (e.g. missing keys).
    pub unresolved: Vec<String>,
}

/// Resolves `{{profile.key}}` and `{{data.key}}` placeholders by looking up
/// values in the database.
pub struct PlaceholderResolver {
    pool: PgPool,
}

impl PlaceholderResolver {
    /// Create a new resolver backed by the given database pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Resolve all `{{profile.*}}` and `{{data.*}}` placeholders in `text`.
    ///
    /// - `profile.*` placeholders are resolved from `accounts.profile_data`.
    /// - `data.*` placeholders are resolved from `data_entries.values` for the given task.
    ///
    /// Any placeholders that cannot be resolved are left in the text unchanged and
    /// returned in `unresolved`.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    pub async fn resolve(
        &self,
        text: &str,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<ResolveResult, sqlx::Error> {
        let Some(re) = Regex::new(r"\{\{(profile|data)\.([\w]+)\}\}").ok() else {
            return Ok(ResolveResult {
                text: text.to_string(),
                unresolved: Vec::new(),
            });
        };

        let captures: Vec<_> = re.captures_iter(text).collect();
        if captures.is_empty() {
            return Ok(ResolveResult {
                text: text.to_string(),
                unresolved: Vec::new(),
            });
        }

        // Determine which kinds of lookups we need
        let needs_profile = captures.iter().any(|c| &c[1] == "profile");
        let needs_data = captures.iter().any(|c| &c[1] == "data");

        // Fetch profile data if needed
        let profile_data: Option<Value> = if needs_profile {
            let row: Option<(Value,)> =
                sqlx::query_as("SELECT profile_data FROM accounts WHERE id = $1")
                    .bind(account_id)
                    .fetch_optional(&self.pool)
                    .await?;
            row.map(|(v,)| v)
        } else {
            None
        };

        // Fetch data entries if needed — merge all entry values into one map
        let data_values: Option<Value> = if needs_data {
            let rows: Vec<(Value,)> = sqlx::query_as(
                "SELECT values FROM data_entries WHERE task_id = $1 AND deleted_at IS NULL",
            )
            .bind(task_id)
            .fetch_all(&self.pool)
            .await?;

            let mut merged = serde_json::Map::new();
            for (vals,) in rows {
                if let Value::Object(map) = vals {
                    for (k, v) in map {
                        merged.insert(k, v);
                    }
                }
            }
            Some(Value::Object(merged))
        } else {
            None
        };

        let mut unresolved = Vec::new();
        let result = re
            .replace_all(text, |caps: &regex::Captures<'_>| {
                let scope = &caps[1];
                let key = &caps[2];
                let full_match = caps[0].to_string();

                let lookup = match scope {
                    "profile" => profile_data
                        .as_ref()
                        .and_then(|pd| pd.get(key))
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    "data" => data_values
                        .as_ref()
                        .and_then(|dv| dv.get(key))
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    _ => None,
                };

                lookup.unwrap_or_else(|| {
                    unresolved.push(full_match.clone());
                    full_match
                })
            })
            .into_owned();

        Ok(ResolveResult {
            text: result,
            unresolved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_finds_placeholders() {
        let re = Regex::new(r"\{\{(profile|data)\.([\w]+)\}\}").unwrap();

        let text = "Hello {{profile.name}}, your code is {{data.code_field}}!";
        let captures: Vec<_> = re.captures_iter(text).collect();

        assert_eq!(captures.len(), 2);
        assert_eq!(&captures[0][1], "profile");
        assert_eq!(&captures[0][2], "name");
        assert_eq!(&captures[1][1], "data");
        assert_eq!(&captures[1][2], "code_field");
    }

    #[test]
    fn regex_ignores_invalid_placeholders() {
        let re = Regex::new(r"\{\{(profile|data)\.([\w]+)\}\}").unwrap();

        let text = "No match: {{unknown.key}} and {{profile.}} end";
        let captures: Vec<_> = re.captures_iter(text).collect();
        assert!(captures.is_empty());
    }
}
