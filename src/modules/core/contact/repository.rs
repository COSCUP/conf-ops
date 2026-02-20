use sqlx::PgPool;
use uuid::Uuid;

use super::error::ContactError;
use super::models::Contact;

pub struct ContactRepository;

impl ContactRepository {
    /// Insert a new contact into the database.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        organization_id: Uuid,
        name: &str,
        email: &str,
    ) -> Result<Contact, ContactError> {
        sqlx::query_as!(
            Contact,
            r#"INSERT INTO contacts (id, organization_id, name, email)
             VALUES ($1, $2, $3, $4)
             RETURNING id, organization_id, name, email, merged_into_id,
                       created_at, updated_at, deleted_at"#,
            id,
            organization_id,
            name,
            email,
        )
        .fetch_one(pool)
        .await
        .map_err(ContactError::Database)
    }

    /// Fetch a contact by ID.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::NotFound` if the contact does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Contact, ContactError> {
        sqlx::query_as!(
            Contact,
            r#"SELECT id, organization_id, name, email, merged_into_id,
                      created_at, updated_at, deleted_at
             FROM contacts
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ContactError::NotFound)
    }

    /// Update a contact's name and email.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::NotFound` if the contact does not exist.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        email: Option<&str>,
    ) -> Result<Contact, ContactError> {
        let current = Self::get_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let email = email.unwrap_or(&current.email);

        sqlx::query_as!(
            Contact,
            r#"UPDATE contacts
             SET name = $2, email = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, organization_id, name, email, merged_into_id,
                       created_at, updated_at, deleted_at"#,
            id,
            name,
            email,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(ContactError::NotFound)
    }

    /// Soft-delete a contact by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::NotFound` if the contact does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), ContactError> {
        let result = sqlx::query!(
            "UPDATE contacts SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ContactError::NotFound);
        }

        Ok(())
    }

    /// List contacts for an organization, optionally filtered by search term.
    ///
    /// When `search` is provided, it performs ILIKE matching on name and email.
    /// Only returns non-deleted, non-merged contacts.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::Database` on database failure.
    pub async fn list_by_org(
        pool: &PgPool,
        organization_id: Uuid,
        search: Option<&str>,
    ) -> Result<Vec<Contact>, ContactError> {
        let contacts = if let Some(term) = search {
            let pattern = format!("%{term}%");
            sqlx::query_as!(
                Contact,
                r#"SELECT id, organization_id, name, email, merged_into_id,
                          created_at, updated_at, deleted_at
                 FROM contacts
                 WHERE organization_id = $1
                   AND deleted_at IS NULL
                   AND merged_into_id IS NULL
                   AND (name ILIKE $2 OR email ILIKE $2)
                 ORDER BY created_at DESC"#,
                organization_id,
                pattern,
            )
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as!(
                Contact,
                r#"SELECT id, organization_id, name, email, merged_into_id,
                          created_at, updated_at, deleted_at
                 FROM contacts
                 WHERE organization_id = $1
                   AND deleted_at IS NULL
                   AND merged_into_id IS NULL
                 ORDER BY created_at DESC"#,
                organization_id,
            )
            .fetch_all(pool)
            .await?
        };

        Ok(contacts)
    }

    /// Set the `merged_into_id` for a contact.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::NotFound` if the contact does not exist.
    pub async fn set_merged_into(
        pool: &PgPool,
        id: Uuid,
        merged_into_id: Uuid,
    ) -> Result<(), ContactError> {
        let result = sqlx::query!(
            "UPDATE contacts SET merged_into_id = $2, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
            merged_into_id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ContactError::NotFound);
        }

        Ok(())
    }

    /// Follow the `merged_into_id` chain to find the final merge target.
    ///
    /// Includes cycle detection with a maximum of 10 hops.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::MergeCycle` if a cycle is detected or chain exceeds 10 hops.
    /// Returns `ContactError::NotFound` if any contact in the chain does not exist.
    pub async fn get_merge_chain_target(pool: &PgPool, id: Uuid) -> Result<Uuid, ContactError> {
        let mut current_id = id;

        for _ in 0..10 {
            let contact = Self::get_by_id(pool, current_id).await?;

            match contact.merged_into_id {
                Some(next_id) => {
                    if next_id == id {
                        return Err(ContactError::MergeCycle);
                    }
                    current_id = next_id;
                }
                None => return Ok(current_id),
            }
        }

        Err(ContactError::MergeCycle)
    }
}
