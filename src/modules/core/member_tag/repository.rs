use sqlx::PgPool;
use uuid::Uuid;

use super::error::MemberTagError;
use super::models::{
    MemberTag, MemberTagAssignment, MemberTagListItem, TagAssignedContact, TagAssignedMember,
};

pub struct MemberTagRepository;

impl MemberTagRepository {
    /// Insert a new member tag into the database.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<MemberTag, MemberTagError> {
        sqlx::query_as!(
            MemberTag,
            r#"INSERT INTO member_tags (id, project_id, name, description)
             VALUES ($1, $2, $3, $4)
             RETURNING id, project_id, name, description,
                       external_task_creation,
                       created_at, updated_at, deleted_at"#,
            id,
            project_id,
            name,
            description,
        )
        .fetch_one(pool)
        .await
        .map_err(MemberTagError::Database)
    }

    /// Fetch a tag by ID.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<MemberTag, MemberTagError> {
        sqlx::query_as!(
            MemberTag,
            r#"SELECT id, project_id, name, description,
                      external_task_creation,
                      created_at, updated_at, deleted_at
             FROM member_tags
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemberTagError::NotFound)
    }

    /// Update a tag's name and/or description.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<MemberTag, MemberTagError> {
        let current = Self::get_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let description = match description {
            Some(d) => d,
            None => current.description.as_deref(),
        };

        sqlx::query_as!(
            MemberTag,
            r#"UPDATE member_tags
             SET name = $2, description = $3, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, project_id, name, description,
                       external_task_creation,
                       created_at, updated_at, deleted_at"#,
            id,
            name,
            description,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemberTagError::NotFound)
    }

    /// Soft-delete a tag by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), MemberTagError> {
        let result = sqlx::query!(
            "UPDATE member_tags SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MemberTagError::NotFound);
        }

        Ok(())
    }

    /// List tags for a project with assignment counts.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::Database` on database failure.
    pub async fn list_by_project(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<MemberTagListItem>, MemberTagError> {
        let items = sqlx::query_as!(
            MemberTagListItem,
            r#"SELECT mt.id, mt.project_id, mt.name, mt.description,
                      mt.created_at, mt.updated_at,
                      (SELECT COUNT(*) FROM member_tag_assignments mta
                       WHERE mta.tag_id = mt.id AND mta.member_id IS NOT NULL) as "member_count!",
                      (SELECT COUNT(*) FROM member_tag_assignments mta
                       WHERE mta.tag_id = mt.id AND mta.contact_id IS NOT NULL) as "contact_count!"
             FROM member_tags mt
             WHERE mt.project_id = $1 AND mt.deleted_at IS NULL
             ORDER BY mt.created_at ASC"#,
            project_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// Create a tag assignment (member or contact).
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::AssignmentAlreadyExists` if duplicate.
    /// Returns `MemberTagError::Database` on other database failure.
    pub async fn create_assignment(
        pool: &PgPool,
        id: Uuid,
        tag_id: Uuid,
        member_id: Option<Uuid>,
        contact_id: Option<Uuid>,
        project_id: Uuid,
    ) -> Result<MemberTagAssignment, MemberTagError> {
        sqlx::query_as!(
            MemberTagAssignment,
            r#"INSERT INTO member_tag_assignments (id, tag_id, member_id, contact_id, project_id)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, tag_id, member_id, contact_id, project_id, created_at"#,
            id,
            tag_id,
            member_id,
            contact_id,
            project_id,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) => {
                let constraint = db_err.constraint();
                if constraint == Some("uq_tag_assignment_member")
                    || constraint == Some("uq_tag_assignment_contact")
                {
                    MemberTagError::AssignmentAlreadyExists
                } else {
                    MemberTagError::Database(e)
                }
            }
            _ => MemberTagError::Database(e),
        })
    }

    /// Delete an assignment by ID.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::AssignmentNotFound` if the assignment does not exist.
    pub async fn delete_assignment(pool: &PgPool, id: Uuid) -> Result<(), MemberTagError> {
        let result = sqlx::query!("DELETE FROM member_tag_assignments WHERE id = $1", id,)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(MemberTagError::AssignmentNotFound);
        }

        Ok(())
    }

    /// List assigned members for a tag (with account info).
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::Database` on database failure.
    pub async fn list_assigned_members(
        pool: &PgPool,
        tag_id: Uuid,
    ) -> Result<Vec<TagAssignedMember>, MemberTagError> {
        let members = sqlx::query_as!(
            TagAssignedMember,
            r#"SELECT mta.id as assignment_id, m.id as member_id,
                      a.id as account_id, a.name, a.email, a.avatar_url
             FROM member_tag_assignments mta
             JOIN members m ON m.id = mta.member_id
             JOIN accounts a ON a.id = m.account_id
             WHERE mta.tag_id = $1 AND mta.member_id IS NOT NULL
             ORDER BY mta.created_at ASC"#,
            tag_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(members)
    }

    /// List assigned contacts for a tag.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::Database` on database failure.
    pub async fn list_assigned_contacts(
        pool: &PgPool,
        tag_id: Uuid,
    ) -> Result<Vec<TagAssignedContact>, MemberTagError> {
        let contacts = sqlx::query_as!(
            TagAssignedContact,
            r#"SELECT mta.id as assignment_id, c.id as contact_id,
                      c.name, c.email
             FROM member_tag_assignments mta
             JOIN contacts c ON c.id = mta.contact_id
             WHERE mta.tag_id = $1 AND mta.contact_id IS NOT NULL
             ORDER BY mta.created_at ASC"#,
            tag_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(contacts)
    }

    /// List tags assigned to a specific member.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::Database` on database failure.
    pub async fn list_tags_for_member(
        pool: &PgPool,
        member_id: Uuid,
    ) -> Result<Vec<MemberTag>, MemberTagError> {
        let tags = sqlx::query_as!(
            MemberTag,
            r#"SELECT mt.id, mt.project_id, mt.name, mt.description,
                      mt.external_task_creation,
                      mt.created_at, mt.updated_at, mt.deleted_at
             FROM member_tags mt
             JOIN member_tag_assignments mta ON mta.tag_id = mt.id
             WHERE mta.member_id = $1 AND mt.deleted_at IS NULL
             ORDER BY mt.created_at ASC"#,
            member_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(tags)
    }

    /// List tags for multiple members in a batch.
    ///
    /// Returns `(member_id, MemberTag)` pairs for enriching member lists.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::Database` on database failure.
    pub async fn list_tags_for_members_batch(
        pool: &PgPool,
        member_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, MemberTag)>, MemberTagError> {
        let rows = sqlx::query_as!(
            BatchTagRow,
            r#"SELECT mta.member_id as "member_id!",
                      mt.id, mt.project_id, mt.name, mt.description,
                      mt.external_task_creation,
                      mt.created_at, mt.updated_at, mt.deleted_at
             FROM member_tags mt
             JOIN member_tag_assignments mta ON mta.tag_id = mt.id
             WHERE mta.member_id = ANY($1) AND mt.deleted_at IS NULL
             ORDER BY mt.created_at ASC"#,
            member_ids,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.member_id,
                    MemberTag {
                        id: r.id,
                        project_id: r.project_id,
                        name: r.name,
                        description: r.description,
                        external_task_creation: r.external_task_creation,
                        created_at: r.created_at,
                        updated_at: r.updated_at,
                        deleted_at: r.deleted_at,
                    },
                )
            })
            .collect())
    }

    /// Update the `external_task_creation` JSONB field.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist.
    pub async fn update_external_task_creation(
        pool: &PgPool,
        id: Uuid,
        settings: &serde_json::Value,
    ) -> Result<MemberTag, MemberTagError> {
        sqlx::query_as!(
            MemberTag,
            r#"UPDATE member_tags
             SET external_task_creation = $2, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, project_id, name, description,
                       external_task_creation,
                       created_at, updated_at, deleted_at"#,
            id,
            settings,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemberTagError::NotFound)
    }
}

/// Internal row type for batch tag query (includes `member_id`).
#[derive(Debug, sqlx::FromRow)]
struct BatchTagRow {
    pub member_id: Uuid,
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub external_task_creation: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}
