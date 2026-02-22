use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};

use super::error::AiError;
use super::models::{Suggestion, SuggestionDecision, SuggestionGroup};
use super::placeholder::PlaceholderResolver;
use super::repository::{
    AiPipelineRepository, AiSuggestionRepository, UpdateSuggestionDecisionParams,
};

/// Parameters for making a decision on an AI suggestion.
pub struct DecideParams {
    pub message_id: Uuid,
    pub group_id: Uuid,
    pub suggestion_id: Uuid,
    pub decision: SuggestionDecision,
    pub decided_by: Uuid,
    pub last_seen_message_id: Uuid,
    pub modified_parameters: Option<serde_json::Value>,
    pub execution_result: Option<serde_json::Value>,
    pub additional_instructions: Option<String>,
}

/// Internal parameters for decision side-effects to avoid too many function arguments.
struct DecisionSideEffectParams<'a> {
    modified_parameters: Option<&'a serde_json::Value>,
    execution_result: Option<serde_json::Value>,
    additional_instructions: Option<&'a str>,
}

/// Service for handling user decisions on AI suggestions.
pub struct DecisionService {
    pool: PgPool,
    event_bus: EventBus,
    placeholder_resolver: Arc<PlaceholderResolver>,
}

impl DecisionService {
    /// Create a new `DecisionService`.
    pub fn new(
        pool: PgPool,
        event_bus: EventBus,
        placeholder_resolver: Arc<PlaceholderResolver>,
    ) -> Self {
        Self {
            pool,
            event_bus,
            placeholder_resolver,
        }
    }

    /// Validate that no new messages have arrived since `last_seen_message_id`.
    ///
    /// # Errors
    ///
    /// Returns `AiError::StaleConversation` if newer messages exist.
    /// Returns `AiError::Database` on database failure.
    pub async fn validate_not_stale(
        &self,
        task_id: Uuid,
        last_seen_message_id: Uuid,
    ) -> Result<(), AiError> {
        let latest_row: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM messages WHERE task_id = $1 ORDER BY id DESC LIMIT 1")
                .bind(task_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AiError::Database)?;

        if let Some((latest,)) = latest_row {
            if latest > last_seen_message_id {
                return Err(AiError::StaleConversation {
                    latest_message_id: latest,
                });
            }
        }

        Ok(())
    }

    /// Apply a user decision to a specific suggestion in a suggestion group.
    ///
    /// Validates the conversation is not stale, checks the current decision is `Pending`,
    /// updates the decision, and performs any side-effects required by the decision type.
    ///
    /// # Errors
    ///
    /// Returns `AiError::StaleConversation` if newer messages exist.
    /// Returns `AiError::GroupNotFound` if the message or group does not exist.
    /// Returns `AiError::SuggestionNotFound` if the suggestion does not exist.
    /// Returns `AiError::AlreadyDecided` if the suggestion already has a non-pending decision.
    /// Returns `AiError::InvalidTrigger` if `decision` is `Pending`.
    /// Returns `AiError::Database` on database failure.
    pub async fn decide(&self, params: DecideParams) -> Result<(), AiError> {
        let task_id = self.get_task_id_for_message(params.message_id).await?;
        self.validate_not_stale(task_id, params.last_seen_message_id)
            .await?;

        let group = AiSuggestionRepository::get_suggestion_group(
            &self.pool,
            params.message_id,
            params.group_id,
        )
        .await?;
        let suggestion = Self::find_pending_suggestion(&group, params.suggestion_id)?;

        if params.decision == SuggestionDecision::Pending {
            return Err(AiError::InvalidTrigger(
                "cannot set decision to pending".to_string(),
            ));
        }

        let final_execution_result = self
            .apply_decision_side_effects(
                task_id,
                params.decided_by,
                &params.decision,
                suggestion,
                &DecisionSideEffectParams {
                    modified_parameters: params.modified_parameters.as_ref(),
                    execution_result: params.execution_result,
                    additional_instructions: params.additional_instructions.as_deref(),
                },
            )
            .await?;

        AiSuggestionRepository::update_suggestion_decision(
            &self.pool,
            UpdateSuggestionDecisionParams {
                message_id: params.message_id,
                group_id: params.group_id,
                suggestion_id: params.suggestion_id,
                decision: params.decision.as_str().to_string(),
                decided_by: params.decided_by,
                modified_parameters: params.modified_parameters,
                execution_result: final_execution_result,
            },
        )
        .await?;

        self.event_bus.publish(DomainEvent::SuggestionDecided {
            message_id: params.message_id,
            task_id,
            suggestion_id: params.suggestion_id,
            decision: params.decision.to_string(),
        });

        Ok(())
    }

    /// Retrieve the `task_id` for a message, returning `GroupNotFound` if absent.
    async fn get_task_id_for_message(&self, message_id: Uuid) -> Result<Uuid, AiError> {
        sqlx::query_scalar!(r#"SELECT task_id FROM messages WHERE id = $1"#, message_id,)
            .fetch_optional(&self.pool)
            .await
            .map_err(AiError::Database)?
            .ok_or(AiError::GroupNotFound)
    }

    /// Find a suggestion by id and assert it is still `Pending`.
    fn find_pending_suggestion(
        group: &SuggestionGroup,
        suggestion_id: Uuid,
    ) -> Result<&Suggestion, AiError> {
        let suggestion = group
            .suggestions
            .iter()
            .find(|s| s.id == suggestion_id)
            .ok_or(AiError::SuggestionNotFound)?;

        if suggestion.decision != SuggestionDecision::Pending {
            return Err(AiError::AlreadyDecided);
        }

        Ok(suggestion)
    }

    /// Execute any side-effects for the decision and return the execution result value.
    async fn apply_decision_side_effects(
        &self,
        task_id: Uuid,
        account_id: Uuid,
        decision: &SuggestionDecision,
        suggestion: &Suggestion,
        params: &DecisionSideEffectParams<'_>,
    ) -> Result<Option<serde_json::Value>, AiError> {
        let modified_parameters = params.modified_parameters;
        let execution_result = params.execution_result.clone();
        let additional_instructions = params.additional_instructions;
        let mock_success = || {
            Some(serde_json::json!({
                "success": true,
                "note": "Phase 9 will implement actual tool execution"
            }))
        };

        match decision {
            SuggestionDecision::Accept => {
                tracing::info!(
                    suggestion_id = %suggestion.id,
                    tool = %suggestion.tool,
                    parameters = %suggestion.parameters,
                    "AI decision: Accept — mock tool execution"
                );
                Ok(mock_success())
            }
            SuggestionDecision::ModifyAndAccept => {
                let params = modified_parameters.unwrap_or(&suggestion.parameters);
                // Resolve placeholders in parameter values before execution
                let resolved_params = self
                    .resolve_params_placeholders(params, task_id, account_id)
                    .await?;
                tracing::info!(
                    suggestion_id = %suggestion.id,
                    tool = %suggestion.tool,
                    parameters = %resolved_params,
                    "AI decision: ModifyAndAccept — mock tool execution with resolved parameters"
                );
                Ok(mock_success())
            }
            SuggestionDecision::Reject => {
                let reason =
                    additional_instructions.map(|r| serde_json::json!({ "rejectionReason": r }));
                tracing::info!(
                    suggestion_id = %suggestion.id,
                    reason = ?additional_instructions,
                    "AI decision: Reject"
                );
                Ok(reason.or(execution_result))
            }
            SuggestionDecision::ReSuggest => {
                let mut payload = serde_json::json!({ "re_suggest_for": suggestion.id });
                if let Some(instructions) = additional_instructions {
                    payload["additional_instructions"] =
                        serde_json::Value::String(instructions.to_string());
                }
                tracing::info!(
                    suggestion_id = %suggestion.id,
                    "AI decision: ReSuggest — inserting new pipeline event"
                );
                AiPipelineRepository::insert_event(&self.pool, task_id, "manual_request", payload)
                    .await?;
                Ok(execution_result)
            }
            SuggestionDecision::Pending => Ok(execution_result),
        }
    }

    /// Resolve `{{profile.*}}` and `{{data.*}}` placeholders in JSON parameter values.
    async fn resolve_params_placeholders(
        &self,
        params: &serde_json::Value,
        task_id: Uuid,
        account_id: Uuid,
    ) -> Result<serde_json::Value, AiError> {
        match params {
            serde_json::Value::String(s) => {
                let result = self
                    .placeholder_resolver
                    .resolve(s, task_id, account_id)
                    .await
                    .map_err(AiError::Database)?;
                Ok(serde_json::Value::String(result.text))
            }
            serde_json::Value::Object(map) => {
                let mut resolved = serde_json::Map::new();
                for (k, v) in map {
                    let rv =
                        Box::pin(self.resolve_params_placeholders(v, task_id, account_id)).await?;
                    resolved.insert(k.clone(), rv);
                }
                Ok(serde_json::Value::Object(resolved))
            }
            serde_json::Value::Array(arr) => {
                let mut resolved = Vec::with_capacity(arr.len());
                for v in arr {
                    let rv =
                        Box::pin(self.resolve_params_placeholders(v, task_id, account_id)).await?;
                    resolved.push(rv);
                }
                Ok(serde_json::Value::Array(resolved))
            }
            other => Ok(other.clone()),
        }
    }
}
