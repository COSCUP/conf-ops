use axum::http::StatusCode;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::modules::storage::error::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("Message not found")]
    MessageNotFound,

    #[error("Task not found")]
    TaskNotFound,

    #[error("Stale conversation: there are {unseen_count} unseen message(s)")]
    StaleConversation {
        latest_message_id: Uuid,
        unseen_count: i64,
    },

    #[error("Invalid source type")]
    InvalidSourceType,

    #[error("Source not found")]
    SourceNotFound,

    #[error("Invalid message content: {0}")]
    InvalidContent(String),

    #[error("Attachment upload failed: {0}")]
    AttachmentUploadFailed(StorageError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<ConversationError> for ProblemDetails {
    fn from(err: ConversationError) -> Self {
        match err {
            ConversationError::MessageNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Message Not Found").with_detail(err.to_string())
            }
            ConversationError::TaskNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Task Not Found").with_detail(err.to_string())
            }
            ConversationError::StaleConversation {
                latest_message_id,
                unseen_count,
            } => Self::new(StatusCode::CONFLICT, "Stale Conversation")
                .with_detail(err.to_string())
                .with_extensions(serde_json::json!({
                    "latestMessageId": latest_message_id,
                    "unseenCount": unseen_count,
                })),
            ConversationError::InvalidSourceType => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Source Type")
                    .with_detail(err.to_string())
            }
            ConversationError::SourceNotFound => {
                Self::new(StatusCode::BAD_REQUEST, "Source Not Found").with_detail(err.to_string())
            }
            ConversationError::InvalidContent(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Invalid Content").with_detail(err.to_string())
            }
            ConversationError::AttachmentUploadFailed(_) | ConversationError::Database(_) => {
                Self::internal_server_error()
            }
        }
    }
}
