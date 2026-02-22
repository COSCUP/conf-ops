use axum::http::StatusCode;

use crate::api::error::ProblemDetails;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Suggestion not found")]
    SuggestionNotFound,

    #[error("Suggestion group not found")]
    GroupNotFound,

    #[error("Stale conversation: new messages since last seen")]
    StaleConversation { latest_message_id: uuid::Uuid },

    #[error("Suggestion already decided")]
    AlreadyDecided,

    #[error("LLM provider error: {0}")]
    LlmError(String),

    #[error("Invalid trigger: {0}")]
    InvalidTrigger(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<AiError> for ProblemDetails {
    fn from(err: AiError) -> Self {
        match err {
            AiError::SuggestionNotFound | AiError::GroupNotFound => {
                Self::new(StatusCode::NOT_FOUND, "Not Found").with_detail(err.to_string())
            }
            AiError::StaleConversation { latest_message_id } => {
                Self::new(StatusCode::CONFLICT, "Stale Conversation")
                    .with_detail(err.to_string())
                    .with_extensions(serde_json::json!({
                        "latestMessageId": latest_message_id.to_string()
                    }))
            }
            AiError::AlreadyDecided => {
                Self::new(StatusCode::CONFLICT, "Already Decided").with_detail(err.to_string())
            }
            AiError::LlmError(_) => {
                Self::new(StatusCode::BAD_GATEWAY, "LLM Error").with_detail(err.to_string())
            }
            AiError::InvalidTrigger(_) => {
                Self::new(StatusCode::BAD_REQUEST, "Bad Request").with_detail(err.to_string())
            }
            AiError::Database(_) => Self::internal_server_error(),
        }
    }
}
