use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};
use crate::id::generate_id;

use super::error::MemberTagError;
use super::models::{
    MemberTag, MemberTagAssignment, MemberTagListItem, TagAssignedContact, TagAssignedMember,
};
use super::repository::MemberTagRepository;

pub struct MemberTagService {
    pool: PgPool,
    event_bus: EventBus,
}

impl MemberTagService {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
    }

    /// Create a new member tag.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::Database` on database failure.
    pub async fn create_tag(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<MemberTag, MemberTagError> {
        let tag_id = generate_id();
        let tag =
            MemberTagRepository::create(&self.pool, tag_id, project_id, name, description).await?;

        self.event_bus
            .publish(DomainEvent::MemberTagCreated { tag_id, project_id });

        Ok(tag)
    }

    /// Get a tag by ID.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist.
    pub async fn get_tag(&self, tag_id: Uuid) -> Result<MemberTag, MemberTagError> {
        MemberTagRepository::get_by_id(&self.pool, tag_id).await
    }

    /// Update a tag's name and/or description.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist.
    pub async fn update_tag(
        &self,
        tag_id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> Result<MemberTag, MemberTagError> {
        MemberTagRepository::update(&self.pool, tag_id, name, description).await
    }

    /// Soft-delete a tag.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist.
    pub async fn delete_tag(&self, tag_id: Uuid) -> Result<(), MemberTagError> {
        let tag = MemberTagRepository::get_by_id(&self.pool, tag_id).await?;
        MemberTagRepository::soft_delete(&self.pool, tag_id).await?;

        self.event_bus.publish(DomainEvent::MemberTagDeleted {
            tag_id,
            project_id: tag.project_id,
        });

        Ok(())
    }

    /// List tags for a project with assignment counts.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::Database` on database failure.
    pub async fn list_tags(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<MemberTagListItem>, MemberTagError> {
        MemberTagRepository::list_by_project(&self.pool, project_id).await
    }

    /// Assign a tag to a member or contact.
    ///
    /// Exactly one of `member_id` or `contact_id` must be `Some`.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::InvalidAssignment` if both or neither are provided.
    /// Returns `MemberTagError::AssignmentAlreadyExists` on duplicate.
    pub async fn assign(
        &self,
        tag_id: Uuid,
        member_id: Option<Uuid>,
        contact_id: Option<Uuid>,
        project_id: Uuid,
    ) -> Result<MemberTagAssignment, MemberTagError> {
        match (member_id, contact_id) {
            (Some(_), Some(_)) | (None, None) => {
                return Err(MemberTagError::InvalidAssignment(
                    "exactly one of memberId or contactId must be provided".to_string(),
                ));
            }
            _ => {}
        }

        let assignment_id = generate_id();
        let assignment = MemberTagRepository::create_assignment(
            &self.pool,
            assignment_id,
            tag_id,
            member_id,
            contact_id,
            project_id,
        )
        .await?;

        self.event_bus.publish(DomainEvent::TagAssigned {
            tag_id,
            assignment_id,
            project_id,
        });

        Ok(assignment)
    }

    /// Remove a tag assignment.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::AssignmentNotFound` if the assignment does not exist.
    pub async fn unassign(
        &self,
        tag_id: Uuid,
        assignment_id: Uuid,
        project_id: Uuid,
    ) -> Result<(), MemberTagError> {
        MemberTagRepository::delete_assignment(&self.pool, assignment_id).await?;

        self.event_bus.publish(DomainEvent::TagUnassigned {
            tag_id,
            assignment_id,
            project_id,
        });

        Ok(())
    }

    /// Get tag detail: tag + assigned members + assigned contacts.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist.
    pub async fn get_tag_detail(
        &self,
        tag_id: Uuid,
    ) -> Result<(MemberTag, Vec<TagAssignedMember>, Vec<TagAssignedContact>), MemberTagError> {
        let tag = MemberTagRepository::get_by_id(&self.pool, tag_id).await?;
        let members = MemberTagRepository::list_assigned_members(&self.pool, tag_id).await?;
        let contacts = MemberTagRepository::list_assigned_contacts(&self.pool, tag_id).await?;

        Ok((tag, members, contacts))
    }

    /// Update the external task creation settings for a tag.
    ///
    /// # Errors
    ///
    /// Returns `MemberTagError::NotFound` if the tag does not exist.
    pub async fn update_external_task_creation(
        &self,
        tag_id: Uuid,
        settings: &serde_json::Value,
    ) -> Result<MemberTag, MemberTagError> {
        MemberTagRepository::update_external_task_creation(&self.pool, tag_id, settings).await
    }
}
