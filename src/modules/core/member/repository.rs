use sqlx::PgPool;
use uuid::Uuid;

use super::error::MemberError;
use super::models::{Member, MemberRole, MemberWithAccount};

pub struct MemberRepository;

impl MemberRepository {
    /// Insert a new member into the database.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::AlreadyExists` if a member with the same project and account already exists.
    /// Returns `MemberError::Database` on other database failures.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        project_id: Uuid,
        account_id: Uuid,
        role: MemberRole,
    ) -> Result<Member, MemberError> {
        sqlx::query_as!(
            Member,
            r#"INSERT INTO members (id, project_id, account_id, role)
             VALUES ($1, $2, $3, $4)
             RETURNING id, project_id, account_id,
                       role as "role: MemberRole",
                       created_at, updated_at, deleted_at"#,
            id,
            project_id,
            account_id,
            role as MemberRole,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err)
                if db_err.constraint() == Some("uq_members_project_account") =>
            {
                MemberError::AlreadyExists
            }
            _ => MemberError::Database(e),
        })
    }

    /// Fetch a member by ID.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::NotFound` if the member does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Member, MemberError> {
        sqlx::query_as!(
            Member,
            r#"SELECT id, project_id, account_id,
                      role as "role: MemberRole",
                      created_at, updated_at, deleted_at
             FROM members
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemberError::NotFound)
    }

    /// Look up a member by project and account IDs.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::Database` on database failure.
    pub async fn get_by_project_and_account(
        pool: &PgPool,
        project_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<Member>, MemberError> {
        let member = sqlx::query_as!(
            Member,
            r#"SELECT id, project_id, account_id,
                      role as "role: MemberRole",
                      created_at, updated_at, deleted_at
             FROM members
             WHERE project_id = $1 AND account_id = $2 AND deleted_at IS NULL"#,
            project_id,
            account_id,
        )
        .fetch_optional(pool)
        .await?;

        Ok(member)
    }

    /// List members with account info (read-model JOIN), optionally filtered by role.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::Database` if the query fails.
    pub async fn list_by_project(
        pool: &PgPool,
        project_id: Uuid,
        role_filter: Option<MemberRole>,
    ) -> Result<Vec<MemberWithAccount>, MemberError> {
        let members = if let Some(role) = role_filter {
            sqlx::query_as!(
                MemberWithAccount,
                r#"SELECT m.id, m.project_id, m.account_id,
                          m.role as "role: MemberRole",
                          m.created_at, m.updated_at,
                          a.name, a.email, a.avatar_url
                 FROM members m
                 JOIN accounts a ON a.id = m.account_id
                 WHERE m.project_id = $1 AND m.role = $2 AND m.deleted_at IS NULL
                 ORDER BY m.created_at ASC"#,
                project_id,
                role as MemberRole,
            )
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as!(
                MemberWithAccount,
                r#"SELECT m.id, m.project_id, m.account_id,
                          m.role as "role: MemberRole",
                          m.created_at, m.updated_at,
                          a.name, a.email, a.avatar_url
                 FROM members m
                 JOIN accounts a ON a.id = m.account_id
                 WHERE m.project_id = $1 AND m.deleted_at IS NULL
                 ORDER BY m.created_at ASC"#,
                project_id,
            )
            .fetch_all(pool)
            .await?
        };

        Ok(members)
    }

    /// Update a member's role.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::NotFound` if the member does not exist.
    pub async fn update_role(
        pool: &PgPool,
        id: Uuid,
        role: MemberRole,
    ) -> Result<Member, MemberError> {
        sqlx::query_as!(
            Member,
            r#"UPDATE members
             SET role = $2, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, project_id, account_id,
                       role as "role: MemberRole",
                       created_at, updated_at, deleted_at"#,
            id,
            role as MemberRole,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(MemberError::NotFound)
    }

    /// Soft-delete a member by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::NotFound` if the member does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), MemberError> {
        let result = sqlx::query!(
            "UPDATE members SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MemberError::NotFound);
        }

        Ok(())
    }

    /// Count the number of owners in a project.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::Database` on database failure.
    pub async fn count_owners(pool: &PgPool, project_id: Uuid) -> Result<i64, MemberError> {
        let record = sqlx::query!(
            r#"SELECT COUNT(*) as "count!" FROM members
             WHERE project_id = $1 AND role = 'owner' AND deleted_at IS NULL"#,
            project_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(record.count)
    }
}
