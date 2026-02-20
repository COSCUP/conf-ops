use sqlx::PgPool;
use uuid::Uuid;

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
}

#[derive(Debug, Clone)]
pub enum Resource {
    Organization { org_id: Uuid },
    Project { org_id: Uuid },
}

pub struct PermissionService {
    pool: PgPool,
}

impl PermissionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Check whether the given account has permission to perform an action on a resource.
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
        let org_id = match resource {
            Resource::Organization { org_id } | Resource::Project { org_id } => *org_id,
        };

        let member = OrgMemberRepository::get_by_org_and_account(&self.pool, org_id, account_id)
            .await
            .map_err(|e| match e {
                crate::modules::core::organization::error::OrgError::Database(db_err) => db_err,
                _ => sqlx::Error::RowNotFound,
            })?;

        let role = match member {
            Some(m) => m.role,
            None => return Ok(false),
        };

        Ok(is_allowed(role, action))
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
    }
}
