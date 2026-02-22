use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Trigger Type ────────────────────────────────────────────

/// The event type that triggered an AI suggestion pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    TaskCreated,
    TodoCompleted,
    MessageSent,
    ToolError,
    SourceDataChanged,
    ManualRequest,
}

impl TriggerType {
    /// Returns the string representation of this trigger type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TaskCreated => "task_created",
            Self::TodoCompleted => "todo_completed",
            Self::MessageSent => "message_sent",
            Self::ToolError => "tool_error",
            Self::SourceDataChanged => "source_data_changed",
            Self::ManualRequest => "manual_request",
        }
    }
}

impl std::fmt::Display for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for TriggerType {
    type Err = String;

    /// Parses a string into a `TriggerType`.
    ///
    /// # Errors
    ///
    /// Returns an error string if the input does not match any known trigger type.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "task_created" => Ok(Self::TaskCreated),
            "todo_completed" => Ok(Self::TodoCompleted),
            "message_sent" => Ok(Self::MessageSent),
            "tool_error" => Ok(Self::ToolError),
            "source_data_changed" => Ok(Self::SourceDataChanged),
            "manual_request" => Ok(Self::ManualRequest),
            other => Err(format!("Invalid trigger type: {other}")),
        }
    }
}

// ── Suggestion Decision ─────────────────────────────────────

/// The decision made on an AI suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionDecision {
    Pending,
    Accept,
    ModifyAndAccept,
    Reject,
    ReSuggest,
}

impl SuggestionDecision {
    /// Returns the string representation of this suggestion decision.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accept => "accept",
            Self::ModifyAndAccept => "modify_and_accept",
            Self::Reject => "reject",
            Self::ReSuggest => "re_suggest",
        }
    }
}

impl std::fmt::Display for SuggestionDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SuggestionDecision {
    type Err = String;

    /// Parses a string into a `SuggestionDecision`.
    ///
    /// # Errors
    ///
    /// Returns an error string if the input does not match any known decision.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "accept" => Ok(Self::Accept),
            "modify_and_accept" => Ok(Self::ModifyAndAccept),
            "reject" => Ok(Self::Reject),
            "re_suggest" => Ok(Self::ReSuggest),
            other => Err(format!("Invalid suggestion decision: {other}")),
        }
    }
}

// ── Suggestion Structures ───────────────────────────────────

/// A group of AI-generated suggestions sharing the same trigger event.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionGroup {
    pub id: Uuid,
    pub trigger: TriggerType,
    pub suggestions: Vec<Suggestion>,
    pub created_at: DateTime<Utc>,
}

/// A single AI suggestion proposing a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: Uuid,
    pub summary: String,
    pub tool: String,
    pub parameters: serde_json::Value,
    pub reasoning: String,
    #[serde(default)]
    pub context_used: Vec<SuggestionContextRef>,
    pub decision: SuggestionDecision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_parameters: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_result: Option<serde_json::Value>,
}

/// A reference to a memory entry used as context for generating a suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionContextRef {
    pub scope_type: String,
    pub memory_id: Uuid,
    pub content: String,
}

// ── AI Context ──────────────────────────────────────────────

/// A record of a single AI LLM call with prompt, response, and token usage.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AiContext {
    pub id: Uuid,
    pub task_id: Uuid,
    pub message_id: Uuid,
    pub prompt: String,
    pub response: String,
    pub model: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub duration_ms: i32,
    pub created_at: DateTime<Utc>,
}

// ── Pipeline Event Status ───────────────────────────────────

/// The processing status of an AI pipeline event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineEventStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl PipelineEventStatus {
    /// Returns the string representation of this pipeline event status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for PipelineEventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PipelineEventStatus {
    type Err = String;

    /// Parses a string into a `PipelineEventStatus`.
    ///
    /// # Errors
    ///
    /// Returns an error string if the input does not match any known status.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            other => Err(format!("Invalid pipeline event status: {other}")),
        }
    }
}

// ── Pipeline Event ──────────────────────────────────────────

/// A queued AI pipeline event with retry logic.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AiPipelineEvent {
    pub id: Uuid,
    pub task_id: Uuid,
    pub trigger_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ── AI Suggestion Content (for message content JSONB) ───────

/// The content structure stored in messages with `source_type = 'ai_suggestion'`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestionContent {
    pub suggestion_group: SuggestionGroup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_type_serde_roundtrip() {
        let variants = vec![
            (TriggerType::TaskCreated, "\"task_created\""),
            (TriggerType::TodoCompleted, "\"todo_completed\""),
            (TriggerType::MessageSent, "\"message_sent\""),
            (TriggerType::ToolError, "\"tool_error\""),
            (TriggerType::SourceDataChanged, "\"source_data_changed\""),
            (TriggerType::ManualRequest, "\"manual_request\""),
        ];

        for (variant, expected_json) in variants {
            let serialized = serde_json::to_string(&variant).expect("should serialize");
            assert_eq!(serialized, expected_json);

            let deserialized: TriggerType =
                serde_json::from_str(&serialized).expect("should deserialize");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn trigger_type_display_and_from_str() {
        let variants = vec![
            (TriggerType::TaskCreated, "task_created"),
            (TriggerType::TodoCompleted, "todo_completed"),
            (TriggerType::MessageSent, "message_sent"),
            (TriggerType::ToolError, "tool_error"),
            (TriggerType::SourceDataChanged, "source_data_changed"),
            (TriggerType::ManualRequest, "manual_request"),
        ];

        for (variant, expected_str) in variants {
            assert_eq!(variant.to_string(), expected_str);
            let parsed: TriggerType = expected_str.parse().expect("should parse");
            assert_eq!(parsed, variant);
        }

        let err = "invalid".parse::<TriggerType>();
        assert!(err.is_err());
    }

    #[test]
    fn suggestion_decision_serde_roundtrip() {
        let variants = vec![
            (SuggestionDecision::Pending, "\"pending\""),
            (SuggestionDecision::Accept, "\"accept\""),
            (SuggestionDecision::ModifyAndAccept, "\"modify_and_accept\""),
            (SuggestionDecision::Reject, "\"reject\""),
            (SuggestionDecision::ReSuggest, "\"re_suggest\""),
        ];

        for (variant, expected_json) in variants {
            let serialized = serde_json::to_string(&variant).expect("should serialize");
            assert_eq!(serialized, expected_json);

            let deserialized: SuggestionDecision =
                serde_json::from_str(&serialized).expect("should deserialize");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn suggestion_decision_display_and_from_str() {
        let variants = vec![
            (SuggestionDecision::Pending, "pending"),
            (SuggestionDecision::Accept, "accept"),
            (SuggestionDecision::ModifyAndAccept, "modify_and_accept"),
            (SuggestionDecision::Reject, "reject"),
            (SuggestionDecision::ReSuggest, "re_suggest"),
        ];

        for (variant, expected_str) in variants {
            assert_eq!(variant.to_string(), expected_str);
            let parsed: SuggestionDecision = expected_str.parse().expect("should parse");
            assert_eq!(parsed, variant);
        }

        let err = "invalid".parse::<SuggestionDecision>();
        assert!(err.is_err());
    }

    #[test]
    fn suggestion_group_serde_roundtrip() {
        let group = SuggestionGroup {
            id: Uuid::nil(),
            trigger: TriggerType::TaskCreated,
            suggestions: vec![Suggestion {
                id: Uuid::nil(),
                summary: "Send welcome email".to_string(),
                tool: "email.send".to_string(),
                parameters: serde_json::json!({"to": "user@example.com"}),
                reasoning: "New task created, should notify stakeholders".to_string(),
                context_used: vec![SuggestionContextRef {
                    scope_type: "task".to_string(),
                    memory_id: Uuid::nil(),
                    content: "Task context summary".to_string(),
                }],
                decision: SuggestionDecision::Pending,
                decided_by: None,
                decided_at: None,
                modified_parameters: None,
                execution_result: None,
            }],
            created_at: DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp"),
        };

        let content = AiSuggestionContent {
            suggestion_group: group,
        };

        let serialized = serde_json::to_string(&content).expect("should serialize");
        let deserialized: AiSuggestionContent =
            serde_json::from_str(&serialized).expect("should deserialize");

        assert_eq!(
            deserialized.suggestion_group.trigger,
            TriggerType::TaskCreated
        );
        assert_eq!(deserialized.suggestion_group.suggestions.len(), 1);
        assert_eq!(
            deserialized.suggestion_group.suggestions[0].decision,
            SuggestionDecision::Pending
        );
        assert!(deserialized.suggestion_group.suggestions[0]
            .decided_by
            .is_none());

        // Verify camelCase field names in JSON
        let json_value: serde_json::Value =
            serde_json::from_str(&serialized).expect("should parse as Value");
        assert!(json_value.get("suggestionGroup").is_some());
        let sg = &json_value["suggestionGroup"];
        assert!(sg.get("createdAt").is_some());
        assert!(sg["suggestions"][0].get("contextUsed").is_some());
        assert!(sg["suggestions"][0].get("decidedBy").is_none()); // skipped when None
    }

    #[test]
    fn suggestion_group_with_decided_suggestion() {
        let decided_at = DateTime::from_timestamp(1_700_001_000, 0).expect("valid timestamp");
        let decider_id = Uuid::from_u128(42);

        let group = SuggestionGroup {
            id: Uuid::nil(),
            trigger: TriggerType::MessageSent,
            suggestions: vec![Suggestion {
                id: Uuid::nil(),
                summary: "Update task status".to_string(),
                tool: "task.update_status".to_string(),
                parameters: serde_json::json!({"status": "in_progress"}),
                reasoning: "User requested status change".to_string(),
                context_used: vec![],
                decision: SuggestionDecision::ModifyAndAccept,
                decided_by: Some(decider_id),
                decided_at: Some(decided_at),
                modified_parameters: Some(serde_json::json!({"status": "completed"})),
                execution_result: Some(serde_json::json!({"success": true})),
            }],
            created_at: DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp"),
        };

        let serialized = serde_json::to_string(&group).expect("should serialize");
        let deserialized: SuggestionGroup =
            serde_json::from_str(&serialized).expect("should deserialize");

        assert_eq!(
            deserialized.suggestions[0].decision,
            SuggestionDecision::ModifyAndAccept
        );
        assert_eq!(deserialized.suggestions[0].decided_by, Some(decider_id));
        assert_eq!(deserialized.suggestions[0].decided_at, Some(decided_at));
        assert!(deserialized.suggestions[0].modified_parameters.is_some());
        assert!(deserialized.suggestions[0].execution_result.is_some());
    }

    #[test]
    fn pipeline_event_status_serde_roundtrip() {
        let variants = vec![
            (PipelineEventStatus::Pending, "\"pending\""),
            (PipelineEventStatus::Processing, "\"processing\""),
            (PipelineEventStatus::Completed, "\"completed\""),
            (PipelineEventStatus::Failed, "\"failed\""),
        ];

        for (variant, expected_json) in variants {
            let serialized = serde_json::to_string(&variant).expect("should serialize");
            assert_eq!(serialized, expected_json);

            let deserialized: PipelineEventStatus =
                serde_json::from_str(&serialized).expect("should deserialize");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn pipeline_event_status_display_and_from_str() {
        let variants = vec![
            (PipelineEventStatus::Pending, "pending"),
            (PipelineEventStatus::Processing, "processing"),
            (PipelineEventStatus::Completed, "completed"),
            (PipelineEventStatus::Failed, "failed"),
        ];

        for (variant, expected_str) in variants {
            assert_eq!(variant.to_string(), expected_str);
            let parsed: PipelineEventStatus = expected_str.parse().expect("should parse");
            assert_eq!(parsed, variant);
        }

        let err = "invalid".parse::<PipelineEventStatus>();
        assert!(err.is_err());
    }
}
