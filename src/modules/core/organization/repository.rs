use sqlx::PgPool;
use uuid::Uuid;

use super::error::OrgError;
use super::models::{MyOrgItem, OrgMemberWithAccount, OrgRole, Organization, OrganizationMember};

pub struct OrganizationRepository;

impl OrganizationRepository {
    /// Insert a new organization into the database.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::Database` on database failure.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        name: &str,
        description: Option<&str>,
        logo_url: Option<&str>,
        created_by: Uuid,
    ) -> Result<Organization, OrgError> {
        sqlx::query_as!(
            Organization,
            r#"INSERT INTO organizations (id, name, description, logo_url, created_by)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, name, description, logo_url, created_by,
                       created_at, updated_at, deleted_at"#,
            id,
            name,
            description,
            logo_url,
            created_by,
        )
        .fetch_one(pool)
        .await
        .map_err(OrgError::Database)
    }

    /// Fetch an organization by ID.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::NotFound` if the organization does not exist or is deleted.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Organization, OrgError> {
        sqlx::query_as!(
            Organization,
            r#"SELECT id, name, description, logo_url, created_by,
                      created_at, updated_at, deleted_at
             FROM organizations
             WHERE id = $1 AND deleted_at IS NULL"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(OrgError::NotFound)
    }

    /// Update an organization's fields.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::NotFound` if the organization does not exist.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
        logo_url: Option<Option<&str>>,
    ) -> Result<Organization, OrgError> {
        let current = Self::get_by_id(pool, id).await?;

        let name = name.unwrap_or(&current.name);
        let description = match description {
            Some(d) => d,
            None => current.description.as_deref(),
        };
        let logo_url = match logo_url {
            Some(l) => l,
            None => current.logo_url.as_deref(),
        };

        sqlx::query_as!(
            Organization,
            r#"UPDATE organizations
             SET name = $2, description = $3, logo_url = $4, updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL
             RETURNING id, name, description, logo_url, created_by,
                       created_at, updated_at, deleted_at"#,
            id,
            name,
            description,
            logo_url,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(OrgError::NotFound)
    }

    /// Soft-delete an organization by setting `deleted_at`.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::NotFound` if the organization does not exist.
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), OrgError> {
        let result = sqlx::query!(
            "UPDATE organizations SET deleted_at = NOW(), updated_at = NOW()
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(OrgError::NotFound);
        }

        Ok(())
    }
}

pub struct OrgMemberRepository;

impl OrgMemberRepository {
    /// Insert a new organization member.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::MemberAlreadyExists` if the account is already a member.
    pub async fn create(
        pool: &PgPool,
        id: Uuid,
        organization_id: Uuid,
        account_id: Uuid,
        role: OrgRole,
    ) -> Result<OrganizationMember, OrgError> {
        sqlx::query_as!(
            OrganizationMember,
            r#"INSERT INTO organization_members (id, organization_id, account_id, role)
             VALUES ($1, $2, $3, $4)
             RETURNING id, organization_id, account_id,
                       role as "role: OrgRole",
                       created_at, updated_at"#,
            id,
            organization_id,
            account_id,
            role as OrgRole,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err)
                if db_err.constraint() == Some("uq_organization_members_org_account") =>
            {
                OrgError::MemberAlreadyExists
            }
            _ => OrgError::Database(e),
        })
    }

    /// Fetch an organization member by ID.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::MemberNotFound` if the member does not exist.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<OrganizationMember, OrgError> {
        sqlx::query_as!(
            OrganizationMember,
            r#"SELECT id, organization_id, account_id,
                      role as "role: OrgRole",
                      created_at, updated_at
             FROM organization_members
             WHERE id = $1"#,
            id,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(OrgError::MemberNotFound)
    }

    /// Look up a member by organization and account IDs.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::Database` on database failure.
    pub async fn get_by_org_and_account(
        pool: &PgPool,
        organization_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<OrganizationMember>, OrgError> {
        let member = sqlx::query_as!(
            OrganizationMember,
            r#"SELECT id, organization_id, account_id,
                      role as "role: OrgRole",
                      created_at, updated_at
             FROM organization_members
             WHERE organization_id = $1 AND account_id = $2"#,
            organization_id,
            account_id,
        )
        .fetch_optional(pool)
        .await?;

        Ok(member)
    }

    /// List members with account info (read-model JOIN).
    ///
    /// This intentionally JOINs `organization_members` with accounts for the member list view.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::Database` if the query fails.
    pub async fn list_by_org(
        pool: &PgPool,
        organization_id: Uuid,
    ) -> Result<Vec<OrgMemberWithAccount>, OrgError> {
        let members = sqlx::query_as!(
            OrgMemberWithAccount,
            r#"SELECT om.id, om.organization_id, om.account_id,
                      om.role as "role: OrgRole",
                      om.created_at, om.updated_at,
                      a.name, a.email, a.avatar_url
             FROM organization_members om
             JOIN accounts a ON a.id = om.account_id
             WHERE om.organization_id = $1
             ORDER BY om.created_at ASC"#,
            organization_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(members)
    }

    /// Update a member's role.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::MemberNotFound` if the member does not exist.
    pub async fn update_role(
        pool: &PgPool,
        id: Uuid,
        role: OrgRole,
    ) -> Result<OrganizationMember, OrgError> {
        sqlx::query_as!(
            OrganizationMember,
            r#"UPDATE organization_members
             SET role = $2, updated_at = NOW()
             WHERE id = $1
             RETURNING id, organization_id, account_id,
                       role as "role: OrgRole",
                       created_at, updated_at"#,
            id,
            role as OrgRole,
        )
        .fetch_optional(pool)
        .await?
        .ok_or(OrgError::MemberNotFound)
    }

    /// Delete an organization member.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::MemberNotFound` if the member does not exist.
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), OrgError> {
        let result = sqlx::query!("DELETE FROM organization_members WHERE id = $1", id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(OrgError::MemberNotFound);
        }

        Ok(())
    }

    /// Count the number of owners in an organization.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::Database` on database failure.
    pub async fn count_owners(pool: &PgPool, organization_id: Uuid) -> Result<i64, OrgError> {
        let record = sqlx::query!(
            r#"SELECT COUNT(*) as "count!" FROM organization_members
             WHERE organization_id = $1 AND role = 'org_owner'"#,
            organization_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(record.count)
    }

    /// List all organizations a given account belongs to.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::Database` on database failure.
    pub async fn list_orgs_for_account(
        pool: &PgPool,
        account_id: Uuid,
    ) -> Result<Vec<MyOrgItem>, OrgError> {
        let orgs = sqlx::query_as!(
            MyOrgItem,
            r#"SELECT o.id, o.name, o.description, o.logo_url, o.created_at,
                      om.role as "role: OrgRole"
             FROM organizations o
             JOIN organization_members om ON om.organization_id = o.id
             WHERE om.account_id = $1 AND o.deleted_at IS NULL
             ORDER BY o.created_at DESC"#,
            account_id,
        )
        .fetch_all(pool)
        .await?;

        Ok(orgs)
    }
}
