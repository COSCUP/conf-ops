use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;
use crate::modules::core::member_tag::repository::MemberTagRepository;
use crate::modules::core::organization::repository::OrgMemberRepository;
use crate::modules::core::project::repository::ProjectRepository;

use super::error::MemberError;
use super::models::{Member, MemberRole, MemberWithAccount};
use super::repository::MemberRepository;

pub struct MemberService {
    pool: PgPool,
    event_bus: EventBus,
}

impl MemberService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
    }

    /// Invite a member to a project.
    ///
    /// Verifies the account is an organization member before creating the project member.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::NotOrgMember` if the account is not an org member.
    /// Returns `MemberError::AlreadyExists` if already a project member.
    pub async fn invite_member(
        &self,
        project_id: Uuid,
        account_id: Uuid,
        role: MemberRole,
        tag_ids: Option<Vec<Uuid>>,
    ) -> Result<Member, MemberError> {
        let org_id = ProjectRepository::get_organization_id(&self.pool, project_id)
            .await
            .map_err(|_| MemberError::NotFound)?;

        let org_member =
            OrgMemberRepository::get_by_org_and_account(&self.pool, org_id, account_id)
                .await
                .map_err(|e| match e {
                    crate::modules::core::organization::error::OrgError::Database(db_err) => {
                        MemberError::Database(db_err)
                    }
                    _ => MemberError::NotFound,
                })?;

        if org_member.is_none() {
            return Err(MemberError::NotOrgMember);
        }

        let member_id = generate_id();
        let member =
            MemberRepository::create(&self.pool, member_id, project_id, account_id, role).await?;

        if let Some(ids) = tag_ids {
            for tag_id in ids {
                let assignment_id = generate_id();
                // Best-effort: ignore errors for individual tag assignments
                let _ = MemberTagRepository::create_assignment(
                    &self.pool,
                    assignment_id,
                    tag_id,
                    Some(member_id),
                    None,
                    project_id,
                )
                .await;
            }
        }

        self.event_bus.publish(DomainEvent::ProjectMemberJoined {
            project_id,
            account_id,
            role: role.to_string(),
        });

        Ok(member)
    }

    /// List members of a project, optionally filtered by role.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::Database` on database failure.
    pub async fn list_members(
        &self,
        project_id: Uuid,
        role_filter: Option<MemberRole>,
    ) -> Result<Vec<MemberWithAccount>, MemberError> {
        MemberRepository::list_by_project(&self.pool, project_id, role_filter).await
    }

    /// Update a member's role.
    ///
    /// If changing from owner, ensures at least one owner remains.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::LastOwnerRemoval` if demoting the last owner.
    pub async fn update_member_role(
        &self,
        member_id: Uuid,
        new_role: MemberRole,
    ) -> Result<Member, MemberError> {
        let current = MemberRepository::get_by_id(&self.pool, member_id).await?;

        if current.role == MemberRole::Owner && new_role != MemberRole::Owner {
            let owner_count =
                MemberRepository::count_owners(&self.pool, current.project_id).await?;
            if owner_count <= 1 {
                return Err(MemberError::LastOwnerRemoval);
            }
        }

        let old_role = current.role.to_string();
        let member = MemberRepository::update_role(&self.pool, member_id, new_role).await?;

        self.event_bus
            .publish(DomainEvent::ProjectMemberRoleChanged {
                project_id: current.project_id,
                account_id: current.account_id,
                old_role,
                new_role: new_role.to_string(),
            });

        Ok(member)
    }

    /// Remove a member from a project.
    ///
    /// If the member is an owner, ensures at least one owner remains.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::LastOwnerRemoval` if removing the last owner.
    pub async fn remove_member(&self, member_id: Uuid) -> Result<(), MemberError> {
        let member = MemberRepository::get_by_id(&self.pool, member_id).await?;

        if member.role == MemberRole::Owner {
            let owner_count = MemberRepository::count_owners(&self.pool, member.project_id).await?;
            if owner_count <= 1 {
                return Err(MemberError::LastOwnerRemoval);
            }
        }

        MemberRepository::soft_delete(&self.pool, member_id).await?;

        self.event_bus.publish(DomainEvent::ProjectMemberRemoved {
            project_id: member.project_id,
            account_id: member.account_id,
        });

        Ok(())
    }

    /// Get a member by ID.
    ///
    /// # Errors
    ///
    /// Returns `MemberError::NotFound` if the member does not exist.
    pub async fn get_member(&self, member_id: Uuid) -> Result<Member, MemberError> {
        MemberRepository::get_by_id(&self.pool, member_id).await
    }
}
