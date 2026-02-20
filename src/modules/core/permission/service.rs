use std::time::Duration;

use moka::future::Cache;
use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::modules::core::member::models::MemberRole;
use crate::modules::core::member::repository::MemberRepository;
use crate::modules::core::organization::models::OrgRole;
use crate::modules::core::organization::repository::OrgMemberRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ViewOrganization,
    UpdateOrganization,
    DeleteOrganization,
    InviteMember,
    RemoveMember,
    UpdateMemberRole,
    CreateProject,
    ViewProject,
    UpdateProject,
    DeleteProject,
    UpdateProjectStatus,
    ViewPermissionSettings,
    UpdatePermissionSettings,
    // Contact operations
    CreateContact,
    UpdateContact,
    DeleteContact,
    MergeContacts,
    // Project member operations
    ViewProjectMembers,
    InviteProjectMember,
    RemoveProjectMember,
    UpdateProjectMemberRole,
    // Tag operations
    ViewTags,
    CreateTag,
    UpdateTag,
    DeleteTag,
    AssignTag,
    UnassignTag,
    UpdateExternalTaskCreation,
}

impl Action {
    fn discriminant(self) -> u8 {
        match self {
            Self::ViewOrganization => 0,
            Self::UpdateOrganization => 1,
            Self::DeleteOrganization => 2,
            Self::InviteMember => 3,
            Self::RemoveMember => 4,
            Self::UpdateMemberRole => 5,
            Self::CreateProject => 6,
            Self::ViewProject => 7,
            Self::UpdateProject => 8,
            Self::DeleteProject => 9,
            Self::UpdateProjectStatus => 10,
            Self::ViewPermissionSettings => 11,
            Self::UpdatePermissionSettings => 12,
            Self::CreateContact => 13,
            Self::UpdateContact => 14,
            Self::DeleteContact => 15,
            Self::MergeContacts => 16,
            Self::ViewProjectMembers => 17,
            Self::InviteProjectMember => 18,
            Self::RemoveProjectMember => 19,
            Self::UpdateProjectMemberRole => 20,
            Self::ViewTags => 21,
            Self::CreateTag => 22,
            Self::UpdateTag => 23,
            Self::DeleteTag => 24,
            Self::AssignTag => 25,
            Self::UnassignTag => 26,
            Self::UpdateExternalTaskCreation => 27,
        }
    }
}

const ACTION_COUNT: u8 = 28;

#[derive(Debug, Clone)]
pub enum Resource {
    Organization { org_id: Uuid },
    Project { org_id: Uuid },
    ProjectScoped { org_id: Uuid, project_id: Uuid },
}

// Cache key: (account_id, resource_discriminant, resource_id, action_discriminant)
//
// resource_discriminant: 0 = Organization, 1 = Project, 2 = ProjectScoped
// For Organization/Project, resource_id = org_id; for ProjectScoped, resource_id = project_id
type CacheKey = (Uuid, u8, Uuid, u8);

pub struct PermissionService {
    pool: PgPool,
    cache: Cache<CacheKey, bool>,
}

impl PermissionService {
    pub fn new(pool: PgPool, event_bus: &EventBus, cache_ttl_secs: u64) -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(cache_ttl_secs))
            .max_capacity(10_000)
            .build();

        let cache_clone = cache.clone();
        let mut rx = event_bus.subscribe();

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                Self::handle_invalidation(&cache_clone, &event).await;
            }
        });

        Self { pool, cache }
    }

    fn make_cache_key(account_id: Uuid, resource: &Resource, action: Action) -> CacheKey {
        match resource {
            Resource::Organization { org_id } => (account_id, 0, *org_id, action.discriminant()),
            Resource::Project { org_id } => (account_id, 1, *org_id, action.discriminant()),
            Resource::ProjectScoped { project_id, .. } => {
                (account_id, 2, *project_id, action.discriminant())
            }
        }
    }

    /// Check whether the given account has permission to perform an action on a resource.
    ///
    /// Results are cached with TTL-based expiration and event-driven invalidation.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` on database failure.
    pub async fn check(
        &self,
        account_id: Uuid,
        resource: &Resource,
        action: Action,
    ) -> Result<bool, sqlx::Error> {
        let cache_key = Self::make_cache_key(account_id, resource, action);

        if let Some(result) = self.cache.get(&cache_key).await {
            return Ok(result);
        }

        let result = self.check_inner(account_id, resource, action).await?;
        self.cache.insert(cache_key, result).await;
        Ok(result)
    }

    async fn check_inner(
        &self,
        account_id: Uuid,
        resource: &Resource,
        action: Action,
    ) -> Result<bool, sqlx::Error> {
        match resource {
            Resource::Organization { org_id } | Resource::Project { org_id } => {
                let org_id = *org_id;

                let member =
                    OrgMemberRepository::get_by_org_and_account(&self.pool, org_id, account_id)
                        .await
                        .map_err(|e| match e {
                            crate::modules::core::organization::error::OrgError::Database(
                                db_err,
                            ) => db_err,
                            _ => sqlx::Error::RowNotFound,
                        })?;

                let role = match member {
                    Some(m) => m.role,
                    None => return Ok(false),
                };

                Ok(is_allowed(role, action))
            }
            Resource::ProjectScoped { org_id, project_id } => {
                let org_id = *org_id;
                let project_id = *project_id;

                // 1. Check org membership first
                let org_member =
                    OrgMemberRepository::get_by_org_and_account(&self.pool, org_id, account_id)
                        .await
                        .map_err(|e| match e {
                            crate::modules::core::organization::error::OrgError::Database(
                                db_err,
                            ) => db_err,
                            _ => sqlx::Error::RowNotFound,
                        })?;

                let org_role = match org_member {
                    Some(m) => m.role,
                    None => return Ok(false),
                };

                // 2. Org owner bypasses everything
                if org_role == OrgRole::OrgOwner {
                    return Ok(true);
                }

                // 3. Org admin: check org-level matrix first
                if org_role == OrgRole::OrgAdmin && is_allowed(org_role, action) {
                    return Ok(true);
                }

                // 4. Check project membership
                let project_member = MemberRepository::get_by_project_and_account(
                    &self.pool, project_id, account_id,
                )
                .await
                .map_err(|e| match e {
                    crate::modules::core::member::error::MemberError::Database(db_err) => db_err,
                    _ => sqlx::Error::RowNotFound,
                })?;

                let project_role = match project_member {
                    Some(m) => m.role,
                    None => return Ok(false),
                };

                // 5. Project owner can do everything project-scoped
                if project_role == MemberRole::Owner {
                    return Ok(true);
                }

                // 6. Check project role matrix
                Ok(is_allowed_project_role(project_role, action))
            }
        }
    }

    async fn handle_invalidation(cache: &Cache<CacheKey, bool>, event: &DomainEvent) {
        match event {
            DomainEvent::ProjectMemberRoleChanged {
                project_id,
                account_id,
                ..
            }
            | DomainEvent::ProjectMemberJoined {
                project_id,
                account_id,
                ..
            }
            | DomainEvent::ProjectMemberRemoved {
                project_id,
                account_id,
                ..
            } => {
                // Invalidate all cached actions for this account in this project
                for action_d in 0..ACTION_COUNT {
                    cache
                        .invalidate(&(*account_id, 2u8, *project_id, action_d))
                        .await;
                }
            }
            DomainEvent::MemberJoined {
                organization_id,
                account_id,
                ..
            } => {
                // Invalidate org-level entries for this account
                for action_d in 0..ACTION_COUNT {
                    cache
                        .invalidate(&(*account_id, 0u8, *organization_id, action_d))
                        .await;
                    cache
                        .invalidate(&(*account_id, 1u8, *organization_id, action_d))
                        .await;
                }
                // Org role change can affect ProjectScoped checks (org role is checked first),
                // but we don't know which project_ids to target. Use invalidate_all as a safe
                // fallback — MemberJoined events are infrequent.
                cache.invalidate_all();
            }
            DomainEvent::TagAssigned { project_id, .. }
            | DomainEvent::TagUnassigned { project_id, .. } => {
                // Tag changes can affect deny-first permissions for the entire project.
                // We don't know which account_ids are affected, so invalidate all entries.
                // Tag assignment events are infrequent enough for this to be acceptable.
                let _ = project_id;
                cache.invalidate_all();
            }
            _ => {}
        }
    }
}

fn is_allowed(role: OrgRole, action: Action) -> bool {
    match role {
        OrgRole::OrgOwner => true,
        OrgRole::OrgAdmin => !matches!(
            action,
            Action::DeleteOrganization
                | Action::UpdateMemberRole
                | Action::ViewPermissionSettings
                | Action::UpdatePermissionSettings
        ),
        OrgRole::OrgMember => matches!(action, Action::ViewOrganization | Action::ViewProject),
    }
}

fn is_allowed_project_role(role: MemberRole, action: Action) -> bool {
    match role {
        MemberRole::Owner => true,
        MemberRole::TagAdmin => matches!(
            action,
            Action::ViewProjectMembers
                | Action::InviteProjectMember
                | Action::ViewTags
                | Action::CreateTag
                | Action::UpdateTag
                | Action::AssignTag
                | Action::UnassignTag
                | Action::UpdateExternalTaskCreation
        ),
        MemberRole::Member => matches!(
            action,
            Action::ViewProjectMembers | Action::ViewTags | Action::CreateTag
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_can_do_everything() {
        let actions = [
            Action::ViewOrganization,
            Action::UpdateOrganization,
            Action::DeleteOrganization,
            Action::InviteMember,
            Action::RemoveMember,
            Action::UpdateMemberRole,
            Action::CreateProject,
            Action::ViewProject,
            Action::UpdateProject,
            Action::DeleteProject,
            Action::UpdateProjectStatus,
            Action::ViewPermissionSettings,
            Action::UpdatePermissionSettings,
            Action::CreateContact,
            Action::UpdateContact,
            Action::DeleteContact,
            Action::MergeContacts,
            Action::ViewProjectMembers,
            Action::InviteProjectMember,
            Action::RemoveProjectMember,
            Action::UpdateProjectMemberRole,
            Action::ViewTags,
            Action::CreateTag,
            Action::UpdateTag,
            Action::DeleteTag,
            Action::AssignTag,
            Action::UnassignTag,
            Action::UpdateExternalTaskCreation,
        ];
        for action in actions {
            assert!(
                is_allowed(OrgRole::OrgOwner, action),
                "Owner should be allowed {action:?}"
            );
        }
    }

    #[test]
    fn admin_permissions() {
        assert!(is_allowed(OrgRole::OrgAdmin, Action::ViewOrganization));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::UpdateOrganization));
        assert!(!is_allowed(OrgRole::OrgAdmin, Action::DeleteOrganization));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::InviteMember));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::RemoveMember));
        assert!(!is_allowed(OrgRole::OrgAdmin, Action::UpdateMemberRole));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::CreateProject));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::ViewProject));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::UpdateProject));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::DeleteProject));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::UpdateProjectStatus));
        assert!(!is_allowed(
            OrgRole::OrgAdmin,
            Action::ViewPermissionSettings
        ));
        assert!(!is_allowed(
            OrgRole::OrgAdmin,
            Action::UpdatePermissionSettings
        ));
        // Contact actions — OrgAdmin can do these
        assert!(is_allowed(OrgRole::OrgAdmin, Action::CreateContact));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::UpdateContact));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::DeleteContact));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::MergeContacts));
        // Project-scoped actions — OrgAdmin can do these at org level
        assert!(is_allowed(OrgRole::OrgAdmin, Action::ViewProjectMembers));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::InviteProjectMember));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::RemoveProjectMember));
        assert!(is_allowed(
            OrgRole::OrgAdmin,
            Action::UpdateProjectMemberRole
        ));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::ViewTags));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::CreateTag));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::UpdateTag));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::DeleteTag));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::AssignTag));
        assert!(is_allowed(OrgRole::OrgAdmin, Action::UnassignTag));
        assert!(is_allowed(
            OrgRole::OrgAdmin,
            Action::UpdateExternalTaskCreation
        ));
    }

    #[test]
    fn member_permissions() {
        assert!(is_allowed(OrgRole::OrgMember, Action::ViewOrganization));
        assert!(!is_allowed(OrgRole::OrgMember, Action::UpdateOrganization));
        assert!(!is_allowed(OrgRole::OrgMember, Action::DeleteOrganization));
        assert!(!is_allowed(OrgRole::OrgMember, Action::InviteMember));
        assert!(!is_allowed(OrgRole::OrgMember, Action::RemoveMember));
        assert!(!is_allowed(OrgRole::OrgMember, Action::UpdateMemberRole));
        assert!(!is_allowed(OrgRole::OrgMember, Action::CreateProject));
        assert!(is_allowed(OrgRole::OrgMember, Action::ViewProject));
        assert!(!is_allowed(OrgRole::OrgMember, Action::UpdateProject));
        assert!(!is_allowed(OrgRole::OrgMember, Action::DeleteProject));
        assert!(!is_allowed(OrgRole::OrgMember, Action::UpdateProjectStatus));
        assert!(!is_allowed(
            OrgRole::OrgMember,
            Action::ViewPermissionSettings
        ));
        assert!(!is_allowed(
            OrgRole::OrgMember,
            Action::UpdatePermissionSettings
        ));
        // Contact actions — OrgMember cannot
        assert!(!is_allowed(OrgRole::OrgMember, Action::CreateContact));
        assert!(!is_allowed(OrgRole::OrgMember, Action::UpdateContact));
        assert!(!is_allowed(OrgRole::OrgMember, Action::DeleteContact));
        assert!(!is_allowed(OrgRole::OrgMember, Action::MergeContacts));
    }

    #[test]
    fn project_owner_can_all() {
        let actions = [
            Action::ViewProjectMembers,
            Action::InviteProjectMember,
            Action::RemoveProjectMember,
            Action::UpdateProjectMemberRole,
            Action::ViewTags,
            Action::CreateTag,
            Action::UpdateTag,
            Action::DeleteTag,
            Action::AssignTag,
            Action::UnassignTag,
            Action::UpdateExternalTaskCreation,
        ];
        for action in actions {
            assert!(
                is_allowed_project_role(MemberRole::Owner, action),
                "Project Owner should be allowed {action:?}"
            );
        }
    }

    #[test]
    fn tag_admin_matrix() {
        // Allowed
        assert!(is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::ViewProjectMembers
        ));
        assert!(is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::InviteProjectMember
        ));
        assert!(is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::ViewTags
        ));
        assert!(is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::CreateTag
        ));
        assert!(is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::UpdateTag
        ));
        assert!(is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::AssignTag
        ));
        assert!(is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::UnassignTag
        ));
        assert!(is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::UpdateExternalTaskCreation
        ));
        // Denied
        assert!(!is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::RemoveProjectMember
        ));
        assert!(!is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::UpdateProjectMemberRole
        ));
        assert!(!is_allowed_project_role(
            MemberRole::TagAdmin,
            Action::DeleteTag
        ));
    }

    #[test]
    fn member_matrix() {
        // Allowed
        assert!(is_allowed_project_role(
            MemberRole::Member,
            Action::ViewProjectMembers
        ));
        assert!(is_allowed_project_role(
            MemberRole::Member,
            Action::ViewTags
        ));
        assert!(is_allowed_project_role(
            MemberRole::Member,
            Action::CreateTag
        ));
        // Denied
        assert!(!is_allowed_project_role(
            MemberRole::Member,
            Action::InviteProjectMember
        ));
        assert!(!is_allowed_project_role(
            MemberRole::Member,
            Action::RemoveProjectMember
        ));
        assert!(!is_allowed_project_role(
            MemberRole::Member,
            Action::UpdateProjectMemberRole
        ));
        assert!(!is_allowed_project_role(
            MemberRole::Member,
            Action::UpdateTag
        ));
        assert!(!is_allowed_project_role(
            MemberRole::Member,
            Action::DeleteTag
        ));
        assert!(!is_allowed_project_role(
            MemberRole::Member,
            Action::AssignTag
        ));
        assert!(!is_allowed_project_role(
            MemberRole::Member,
            Action::UnassignTag
        ));
        assert!(!is_allowed_project_role(
            MemberRole::Member,
            Action::UpdateExternalTaskCreation
        ));
    }

    #[test]
    fn cache_key_different_resources_produce_different_keys() {
        let account_id = Uuid::nil();
        let org_id = Uuid::from_u128(1);
        let project_id = Uuid::from_u128(2);
        let action = Action::ViewOrganization;

        let key_org = PermissionService::make_cache_key(
            account_id,
            &Resource::Organization { org_id },
            action,
        );
        let key_proj =
            PermissionService::make_cache_key(account_id, &Resource::Project { org_id }, action);
        let key_scoped = PermissionService::make_cache_key(
            account_id,
            &Resource::ProjectScoped { org_id, project_id },
            action,
        );

        assert_ne!(
            key_org, key_proj,
            "Organization and Project keys should differ"
        );
        assert_ne!(
            key_org, key_scoped,
            "Organization and ProjectScoped keys should differ"
        );
        assert_ne!(
            key_proj, key_scoped,
            "Project and ProjectScoped keys should differ"
        );
    }

    #[test]
    fn cache_key_different_actions_produce_different_keys() {
        let account_id = Uuid::nil();
        let resource = Resource::Organization {
            org_id: Uuid::from_u128(1),
        };

        let key_view =
            PermissionService::make_cache_key(account_id, &resource, Action::ViewOrganization);
        let key_update =
            PermissionService::make_cache_key(account_id, &resource, Action::UpdateOrganization);

        assert_ne!(key_view, key_update);
    }

    #[test]
    fn cache_key_different_accounts_produce_different_keys() {
        let resource = Resource::Organization {
            org_id: Uuid::from_u128(1),
        };
        let action = Action::ViewOrganization;

        let key_a = PermissionService::make_cache_key(Uuid::from_u128(10), &resource, action);
        let key_b = PermissionService::make_cache_key(Uuid::from_u128(20), &resource, action);

        assert_ne!(key_a, key_b);
    }

    #[test]
    fn action_discriminants_are_unique() {
        let actions = [
            Action::ViewOrganization,
            Action::UpdateOrganization,
            Action::DeleteOrganization,
            Action::InviteMember,
            Action::RemoveMember,
            Action::UpdateMemberRole,
            Action::CreateProject,
            Action::ViewProject,
            Action::UpdateProject,
            Action::DeleteProject,
            Action::UpdateProjectStatus,
            Action::ViewPermissionSettings,
            Action::UpdatePermissionSettings,
            Action::CreateContact,
            Action::UpdateContact,
            Action::DeleteContact,
            Action::MergeContacts,
            Action::ViewProjectMembers,
            Action::InviteProjectMember,
            Action::RemoveProjectMember,
            Action::UpdateProjectMemberRole,
            Action::ViewTags,
            Action::CreateTag,
            Action::UpdateTag,
            Action::DeleteTag,
            Action::AssignTag,
            Action::UnassignTag,
            Action::UpdateExternalTaskCreation,
        ];
        let mut seen = std::collections::HashSet::new();
        for action in actions {
            let d = action.discriminant();
            assert!(seen.insert(d), "Duplicate discriminant {d} for {action:?}");
        }
    }
}
