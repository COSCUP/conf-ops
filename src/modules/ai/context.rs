use std::fmt::Write as _;
use std::sync::Arc;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::error::AiError;
use super::memory::models::InheritedMemories;
use super::memory::service::MemoryService;
use super::models::TriggerType;
use super::privacy::PrivacyEngine;

/// The assembled context ready to be sent to an LLM.
pub struct AssembledContext {
    pub system_prompt: String,
    pub user_prompt: String,
    pub model: String,
}

/// Assembles the full prompt context for an AI pipeline invocation.
pub struct ContextAssembler {
    pool: PgPool,
    memory_service: Arc<MemoryService>,
    privacy_engine: Arc<PrivacyEngine>,
}

impl ContextAssembler {
    /// Create a new `ContextAssembler`.
    pub fn new(
        pool: PgPool,
        memory_service: Arc<MemoryService>,
        privacy_engine: Arc<PrivacyEngine>,
    ) -> Self {
        Self {
            pool,
            memory_service,
            privacy_engine,
        }
    }

    /// Assemble system and user prompts for the given task and trigger.
    ///
    /// Steps:
    /// 1. Fetch inherited memories for the task.
    /// 2. Fetch the last 50 conversation messages for the task.
    /// 3. Mask sensitive values in those messages.
    /// 4. Fetch data schema and profile, mask them (structure only, no values).
    /// 5. Fetch todo progress (total / completed count).
    /// 6. Build system prompt with role, memory context, tools, and output format.
    /// 7. Build user prompt with trigger, conversation, schema, profile, and todo summary.
    ///
    /// # Errors
    ///
    /// Returns `AiError` on memory service, privacy engine, or database failure.
    pub async fn assemble(
        &self,
        task_id: Uuid,
        trigger: &TriggerType,
        account_id: Uuid,
    ) -> Result<AssembledContext, AiError> {
        // 1. Fetch inherited memories
        let inherited = self
            .memory_service
            .get_inherited_memories(task_id, account_id)
            .await
            .map_err(|e| AiError::LlmError(format!("Memory error: {e}")))?;

        // 2. Fetch last 50 messages for the task (source_type, content)
        let raw_rows: Vec<(String, Value)> = sqlx::query_as(
            r"SELECT source_type, content
               FROM messages
               WHERE task_id = $1
               ORDER BY created_at ASC
               LIMIT 50",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        // Convert rows to JSON values matching the shape expected by mask_messages
        let raw_messages: Vec<Value> = raw_rows
            .into_iter()
            .map(|(source_type, content)| {
                serde_json::json!({
                    "sourceType": source_type,
                    "content": content,
                })
            })
            .collect();

        // 3. Mask sensitive values
        let masked_messages = self
            .privacy_engine
            .mask_messages(task_id, account_id, &raw_messages)
            .await?;

        // 4. Fetch data schema and profile, then mask them
        let data_schema_row: Option<(Value,)> = sqlx::query_as(
            r"SELECT ds.fields FROM data_schemas ds
               JOIN tasks t ON t.data_schema_id = ds.id
               WHERE t.id = $1",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        let masked_data_schema =
            data_schema_row.map(|(fields,)| PrivacyEngine::mask_data_schema(&fields));

        let profile_row: Option<(Value,)> =
            sqlx::query_as("SELECT profile_data FROM accounts WHERE id = $1")
                .bind(account_id)
                .fetch_optional(&self.pool)
                .await?;

        let masked_profile = profile_row.map(|(profile,)| PrivacyEngine::mask_profile(&profile));

        // 5. Fetch todo progress
        let todo_counts: (Option<i64>, Option<i64>) = sqlx::query_as(
            r"SELECT COUNT(*) AS total,
                      COUNT(*) FILTER (WHERE status = 'completed') AS completed
               FROM todos
               WHERE task_id = $1 AND deleted_at IS NULL",
        )
        .bind(task_id)
        .fetch_one(&self.pool)
        .await?;

        let total_todos = todo_counts.0.unwrap_or(0);
        let completed_todos = todo_counts.1.unwrap_or(0);

        // 6. Build system prompt
        let system_prompt = build_system_prompt(&inherited);

        // 7. Build user prompt
        let user_prompt = build_user_prompt(
            trigger,
            &masked_messages,
            total_todos,
            completed_todos,
            masked_data_schema.as_ref(),
            masked_profile.as_ref(),
        );

        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string());

        Ok(AssembledContext {
            system_prompt,
            user_prompt,
            model,
        })
    }
}

fn build_system_prompt(inherited: &InheritedMemories) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are an AI assistant for a conference management system.\n\n");

    // Memory context grouped by scope_type
    if !inherited.memories.is_empty() {
        prompt.push_str("## Context Memory\n\n");

        // Group by scope_type, preserving first-seen order
        let mut scope_groups: Vec<(&str, Vec<&str>)> = Vec::new();
        for memory in &inherited.memories {
            let scope = memory.scope_type.as_str();
            if let Some(group) = scope_groups.iter_mut().find(|(s, _)| *s == scope) {
                group.1.push(memory.content.as_str());
            } else {
                scope_groups.push((scope, vec![memory.content.as_str()]));
            }
        }

        for (scope_type, contents) in scope_groups {
            let _ = writeln!(prompt, "### {scope_type}");
            for content in contents {
                let _ = writeln!(prompt, "- {content}");
            }
            prompt.push('\n');
        }
    }

    // Available tools
    prompt.push_str("## Available Tools\n\n");
    let tools = [
        "email.send",
        "todo.create",
        "todo.update_status",
        "data.update",
    ];
    for tool in tools {
        let _ = writeln!(prompt, "- {tool}");
    }
    prompt.push('\n');

    // Output format instructions
    prompt.push_str("## Output Format\n\n");
    prompt.push_str(
        "Return a JSON object in `SuggestionGroup` format with the following structure:\n",
    );
    prompt.push_str(
        r#"{
  "id": "<uuid-v7>",
  "trigger": "<trigger_type>",
  "suggestions": [
    {
      "id": "<uuid-v7>",
      "summary": "<short description>",
      "tool": "<tool name>",
      "parameters": { ... },
      "reasoning": "<why this suggestion is relevant>",
      "contextUsed": [
        { "scopeType": "<scope>", "memoryId": "<uuid>", "content": "<excerpt>" }
      ],
      "decision": "pending"
    }
  ],
  "createdAt": "<ISO 8601 timestamp>"
}
"#,
    );
    prompt.push_str(
        "Return only valid JSON. Do not include markdown code fences or extra commentary.\n",
    );

    prompt
}

fn build_user_prompt(
    trigger: &TriggerType,
    masked_messages: &[Value],
    total_todos: i64,
    completed_todos: i64,
    masked_data_schema: Option<&Value>,
    masked_profile: Option<&Value>,
) -> String {
    let mut prompt = String::new();

    // Trigger description
    prompt.push_str("## Trigger\n\n");
    let trigger_desc = match trigger {
        TriggerType::TaskCreated => "A new task has just been created.",
        TriggerType::TodoCompleted => "A todo item has been marked as completed.",
        TriggerType::MessageSent => {
            "A new message has been sent in the task conversation (by a member or via email)."
        }
        TriggerType::ToolError => "A tool execution has failed.",
        TriggerType::SourceDataChanged => "The task source data has been updated.",
        TriggerType::ManualRequest => "The user has manually requested AI suggestions.",
    };
    prompt.push_str(trigger_desc);
    prompt.push_str("\n\n");

    // Conversation history
    prompt.push_str("## Conversation History\n\n");
    if masked_messages.is_empty() {
        prompt.push_str("No messages yet.\n");
    } else {
        for msg in masked_messages {
            let source_type = msg["sourceType"].as_str().unwrap_or("unknown");
            let content_text = msg["content"]["text"].as_str().unwrap_or_default();
            if content_text.is_empty() {
                // For structured messages (tool_execution, system, ai_suggestion)
                let content_str =
                    serde_json::to_string(&msg["content"]).unwrap_or_else(|_| "{}".to_string());
                let _ = writeln!(prompt, "[{source_type}] {content_str}");
            } else {
                let _ = writeln!(prompt, "[{source_type}] {content_text}");
            }
        }
    }
    prompt.push('\n');

    // Data schema (masked — structure only, no values)
    if let Some(schema) = masked_data_schema {
        prompt.push_str("## Data Schema (structure only)\n\n");
        let schema_str = serde_json::to_string_pretty(schema).unwrap_or_else(|_| "{}".to_string());
        let _ = writeln!(prompt, "{schema_str}");
        prompt.push('\n');
    }

    // Profile schema (masked — field names and types only)
    if let Some(profile) = masked_profile {
        prompt.push_str("## Profile Schema (structure only)\n\n");
        let profile_str =
            serde_json::to_string_pretty(profile).unwrap_or_else(|_| "{}".to_string());
        let _ = writeln!(prompt, "{profile_str}");
        prompt.push('\n');
    }

    // Todo progress summary
    prompt.push_str("## Todo Progress\n\n");
    let _ = writeln!(
        prompt,
        "{completed_todos} of {total_todos} todos completed."
    );

    prompt
}
