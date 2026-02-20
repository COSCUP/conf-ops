use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;
use crate::modules::auth::repository::AccountRepository;
use crate::modules::core::project::repository::ProjectRepository;
use crate::modules::email::EmailService;

use super::error::OrgError;
use super::models::{MyOrgItem, OrgMemberWithAccount, OrgRole, Organization};
use super::repository::{OrgMemberRepository, OrganizationRepository};

pub struct OrganizationService {
    pool: PgPool,
    event_bus: EventBus,
    email_service: Arc<dyn EmailService>,
    frontend_url: String,
}

impl OrganizationService {
    pub fn new(
        pool: PgPool,
        event_bus: EventBus,
        email_service: Arc<dyn EmailService>,
        frontend_url: String,
    ) -> Self {
        Self {
            pool,
            event_bus,
            email_service,
            frontend_url,
        }
    }

    /// Create a new organization and add the creator as owner.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::Database` on database failure.
    pub async fn create_organization(
        &self,
        actor_id: Uuid,
        name: &str,
        description: Option<&str>,
        logo_url: Option<&str>,
    ) -> Result<Organization, OrgError> {
        let org_id = generate_id();
        let org = OrganizationRepository::create(
            &self.pool,
            org_id,
            name,
            description,
            logo_url,
            actor_id,
        )
        .await?;

        let member_id = generate_id();
        OrgMemberRepository::create(&self.pool, member_id, org_id, actor_id, OrgRole::OrgOwner)
            .await?;

        self.event_bus.publish(DomainEvent::MemberJoined {
            organization_id: org_id,
            account_id: actor_id,
        });

        self.event_bus.publish(DomainEvent::OrganizationCreated {
            organization_id: org_id,
            created_by: actor_id,
        });

        Ok(org)
    }

    /// Update an organization's fields.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::NotFound` if the organization does not exist.
    pub async fn update_organization(
        &self,
        org_id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
        logo_url: Option<Option<&str>>,
    ) -> Result<Organization, OrgError> {
        OrganizationRepository::update(&self.pool, org_id, name, description, logo_url).await
    }

    /// Fetch an organization by ID.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::NotFound` if the organization does not exist.
    pub async fn get_organization(&self, org_id: Uuid) -> Result<Organization, OrgError> {
        OrganizationRepository::get_by_id(&self.pool, org_id).await
    }

    /// List all organizations the given account belongs to.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::Database` on database failure.
    pub async fn list_my_organizations(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<MyOrgItem>, OrgError> {
        OrgMemberRepository::list_orgs_for_account(&self.pool, account_id).await
    }

    /// Delete an organization if it has no active projects.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::HasActiveProjects` if the organization still has non-archived projects.
    pub async fn delete_organization(&self, org_id: Uuid) -> Result<(), OrgError> {
        let count = ProjectRepository::count_not_archived_by_org(&self.pool, org_id)
            .await
            .map_err(|_| OrgError::Database(sqlx::Error::RowNotFound))?;

        if count > 0 {
            return Err(OrgError::HasActiveProjects);
        }

        OrganizationRepository::soft_delete(&self.pool, org_id).await
    }

    /// List all members of an organization with their account details.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::Database` on database failure.
    pub async fn list_members(&self, org_id: Uuid) -> Result<Vec<OrgMemberWithAccount>, OrgError> {
        OrgMemberRepository::list_by_org(&self.pool, org_id).await
    }

    /// Invite a member to an organization by email, creating an account if needed.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::MemberAlreadyExists` if the account is already a member.
    pub async fn invite_member(
        &self,
        org_id: Uuid,
        email: &str,
        role: OrgRole,
    ) -> Result<Uuid, OrgError> {
        // Get or create account
        let account = AccountRepository::get_by_email(&self.pool, email)
            .await
            .map_err(|e| {
                OrgError::Database(match e {
                    crate::modules::auth::error::AuthError::Database(db_err) => db_err,
                    _ => sqlx::Error::RowNotFound,
                })
            })?;

        let account_id = if let Some(a) = account {
            a.id
        } else {
            let new_id = generate_id();
            let name = email.split('@').next().unwrap_or("User").to_string();
            AccountRepository::create(&self.pool, new_id, email, &name)
                .await
                .map_err(|e| {
                    OrgError::Database(match e {
                        crate::modules::auth::error::AuthError::Database(db_err) => db_err,
                        _ => sqlx::Error::RowNotFound,
                    })
                })?;

            self.event_bus
                .publish(DomainEvent::AccountCreated { account_id: new_id });

            // Send invitation email
            let login_url = format!("{}/login", self.frontend_url);
            let html_body = format!(
                "<h2>You've been invited to an organization on Conf-Ops</h2>\
                 <p>You have been invited to join an organization. Please log in to accept:</p>\
                 <p><a href=\"{login_url}\">{login_url}</a></p>"
            );
            let _ = self
                .email_service
                .send(email, "Organization Invitation - Conf-Ops", &html_body)
                .await;

            new_id
        };

        let member_id = generate_id();
        OrgMemberRepository::create(&self.pool, member_id, org_id, account_id, role).await?;

        self.event_bus.publish(DomainEvent::MemberJoined {
            organization_id: org_id,
            account_id,
        });

        self.event_bus.publish(DomainEvent::MemberInvited {
            organization_id: org_id,
            account_id,
        });

        Ok(member_id)
    }

    /// Remove a member from an organization.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::LastOwnerRemoval` if attempting to remove the last owner.
    pub async fn remove_member(&self, org_id: Uuid, member_id: Uuid) -> Result<(), OrgError> {
        let member = OrgMemberRepository::get_by_id(&self.pool, member_id).await?;

        if member.organization_id != org_id {
            return Err(OrgError::MemberNotFound);
        }

        if member.role == OrgRole::OrgOwner {
            let owner_count = OrgMemberRepository::count_owners(&self.pool, org_id).await?;
            if owner_count <= 1 {
                return Err(OrgError::LastOwnerRemoval);
            }
        }

        OrgMemberRepository::delete(&self.pool, member_id).await
    }

    /// Update a member's role within an organization.
    ///
    /// # Errors
    ///
    /// Returns `OrgError::LastOwnerRemoval` if demoting the last owner.
    pub async fn update_member_role(
        &self,
        org_id: Uuid,
        member_id: Uuid,
        new_role: OrgRole,
    ) -> Result<(), OrgError> {
        let member = OrgMemberRepository::get_by_id(&self.pool, member_id).await?;

        if member.organization_id != org_id {
            return Err(OrgError::MemberNotFound);
        }

        // If changing from owner to non-owner, check last-owner guard
        if member.role == OrgRole::OrgOwner && new_role != OrgRole::OrgOwner {
            let owner_count = OrgMemberRepository::count_owners(&self.pool, org_id).await?;
            if owner_count <= 1 {
                return Err(OrgError::LastOwnerRemoval);
            }
        }

        OrgMemberRepository::update_role(&self.pool, member_id, new_role).await?;
        Ok(())
    }
}
