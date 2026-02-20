use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;

use super::error::ContactError;
use super::models::Contact;
use super::repository::ContactRepository;

pub struct ContactService {
    pool: PgPool,
    event_bus: EventBus,
}

impl ContactService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
    }

    /// Create a new contact within an organization.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::Database` on database failure.
    pub async fn create_contact(
        &self,
        organization_id: Uuid,
        name: &str,
        email: &str,
    ) -> Result<Contact, ContactError> {
        let contact_id = generate_id();
        let contact =
            ContactRepository::create(&self.pool, contact_id, organization_id, name, email).await?;

        self.event_bus.publish(DomainEvent::ContactCreated {
            contact_id,
            organization_id,
        });

        Ok(contact)
    }

    /// Get a contact by ID.
    ///
    /// If the contact has been merged, follows the merge chain and returns the
    /// final target contact.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::NotFound` if the contact does not exist.
    pub async fn get_contact(&self, contact_id: Uuid) -> Result<Contact, ContactError> {
        let contact = ContactRepository::get_by_id(&self.pool, contact_id).await?;

        if contact.merged_into_id.is_some() {
            let target_id =
                ContactRepository::get_merge_chain_target(&self.pool, contact_id).await?;
            return ContactRepository::get_by_id(&self.pool, target_id).await;
        }

        Ok(contact)
    }

    /// Update a contact's name and email.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::NotFound` if the contact does not exist.
    pub async fn update_contact(
        &self,
        contact_id: Uuid,
        name: Option<&str>,
        email: Option<&str>,
    ) -> Result<Contact, ContactError> {
        ContactRepository::update(&self.pool, contact_id, name, email).await
    }

    /// Soft-delete a contact.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::NotFound` if the contact does not exist.
    pub async fn delete_contact(&self, contact_id: Uuid) -> Result<(), ContactError> {
        ContactRepository::soft_delete(&self.pool, contact_id).await
    }

    /// List contacts for an organization, optionally filtered by search term.
    ///
    /// # Errors
    ///
    /// Returns `ContactError::Database` on database failure.
    pub async fn list_contacts(
        &self,
        organization_id: Uuid,
        search: Option<&str>,
    ) -> Result<Vec<Contact>, ContactError> {
        ContactRepository::list_by_org(&self.pool, organization_id, search).await
    }

    /// Merge multiple source contacts into a target contact.
    ///
    /// Validates that:
    /// - Target is not in source IDs
    /// - All contacts belong to the same organization
    /// - Target is not already merged
    /// - No cycles would be created
    ///
    /// # Errors
    ///
    /// Returns appropriate `ContactError` variants on validation failure.
    pub async fn merge_contacts(
        &self,
        organization_id: Uuid,
        source_ids: Vec<Uuid>,
        target_id: Uuid,
    ) -> Result<usize, ContactError> {
        if source_ids.contains(&target_id) {
            return Err(ContactError::SourceContainsTarget);
        }

        let target = ContactRepository::get_by_id(&self.pool, target_id).await?;

        if target.organization_id != organization_id {
            return Err(ContactError::CrossOrgMerge);
        }

        if target.merged_into_id.is_some() {
            return Err(ContactError::AlreadyMerged);
        }

        for source_id in &source_ids {
            let source = ContactRepository::get_by_id(&self.pool, *source_id).await?;

            if source.organization_id != organization_id {
                return Err(ContactError::CrossOrgMerge);
            }

            if source.merged_into_id.is_some() {
                return Err(ContactError::AlreadyMerged);
            }
        }

        let merged_count = source_ids.len();

        for source_id in &source_ids {
            ContactRepository::set_merged_into(&self.pool, *source_id, target_id).await?;
        }

        self.event_bus.publish(DomainEvent::ContactsMerged {
            target_id,
            source_ids,
            organization_id,
        });

        Ok(merged_count)
    }
}
