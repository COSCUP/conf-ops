use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::events::{DomainEvent, EventBus};

use super::context::ContextAssembler;
use super::error::AiError;
use super::llm_client::LlmProvider;
use super::models::{AiPipelineEvent, AiSuggestionContent, SuggestionGroup, TriggerType};
use super::repository::InsertAiContextParams;
use super::repository::{AiContextRepository, AiPipelineRepository, AiSuggestionRepository};

/// Background worker that polls and processes AI pipeline events.
pub struct PipelineWorker {
    pool: PgPool,
    event_bus: EventBus,
    context_assembler: Arc<ContextAssembler>,
    llm_provider: Arc<dyn LlmProvider>,
}

impl PipelineWorker {
    /// Create a new `PipelineWorker`.
    pub fn new(
        pool: PgPool,
        event_bus: EventBus,
        context_assembler: Arc<ContextAssembler>,
        llm_provider: Arc<dyn LlmProvider>,
    ) -> Self {
        Self {
            pool,
            event_bus,
            context_assembler,
            llm_provider,
        }
    }

    /// Spawn the worker loop in a background task.
    ///
    /// On startup, resets any stale events stuck in `processing` for more than 5 minutes.
    /// Then subscribes to the `EventBus` to be notified of new events, with a 2-second
    /// fallback poll interval to ensure no events are missed.
    pub fn start(self: Arc<Self>) -> JoinHandle<()> {
        let mut rx = self.event_bus.subscribe();

        tokio::spawn(async move {
            // Recover events stuck in processing on startup
            match AiPipelineRepository::reset_stale(&self.pool, 300).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("AI pipeline: reset {n} stale event(s) on startup"),
                Err(e) => tracing::warn!("AI pipeline: failed to reset stale events: {e}"),
            }

            tracing::info!("AI pipeline: EventBus subscription established");

            loop {
                // Process all available events first
                self.drain_pending_events().await;

                // Wait for EventBus notification or fallback timeout (2 seconds)
                tokio::select! {
                    result = rx.recv() => {
                        match result {
                            Ok(_) => {} // New event available, loop back to drain
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!("AI pipeline: EventBus lagged, skipped {n} events");
                                // Still drain pending — they are persisted in the DB
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!("AI pipeline: EventBus closed, stopping worker");
                                break;
                            }
                        }
                    }
                    () = tokio::time::sleep(Duration::from_secs(2)) => {
                        // Fallback poll to catch any missed notifications
                    }
                }
            }
        })
    }

    /// Drain all pending events from the database.
    async fn drain_pending_events(&self) {
        loop {
            match AiPipelineRepository::claim_next(&self.pool).await {
                Ok(Some(event)) => {
                    let event_id = event.id;
                    tracing::info!(
                        event_id = %event_id,
                        task_id = %event.task_id,
                        trigger_type = %event.trigger_type,
                        "AI pipeline: processing event"
                    );
                    if let Err(e) = self.process_event(event).await {
                        tracing::error!(
                            event_id = %event_id,
                            error = %e,
                            "AI pipeline: event processing failed"
                        );
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    tracing::error!(error = %e, "AI pipeline: failed to claim next event");
                    break;
                }
            }
        }
    }

    /// Process a single AI pipeline event.
    ///
    /// # Errors
    ///
    /// Returns `AiError` on any failure; also calls `fail_event` before returning.
    pub async fn process_event(&self, event: AiPipelineEvent) -> Result<(), AiError> {
        let result = self.do_process_event(&event).await;

        if let Err(ref e) = result {
            let error_msg = e.to_string();
            if let Err(fail_err) =
                AiPipelineRepository::fail_event(&self.pool, event.id, &error_msg).await
            {
                tracing::error!(
                    event_id = %event.id,
                    error = %fail_err,
                    "AI pipeline: failed to record event failure"
                );
            }
        }

        result
    }

    async fn do_process_event(&self, event: &AiPipelineEvent) -> Result<(), AiError> {
        let task_id = event.task_id;

        // Parse trigger type
        let trigger_type: TriggerType = event
            .trigger_type
            .parse()
            .map_err(AiError::InvalidTrigger)?;

        // Get the account_id from the task's created_by
        let account_id =
            sqlx::query_scalar!(r#"SELECT created_by FROM tasks WHERE id = $1"#, task_id,)
                .fetch_optional(&self.pool)
                .await
                .map_err(AiError::Database)?
                .ok_or_else(|| {
                    AiError::LlmError(format!("Task {task_id} not found or has no creator"))
                })?;

        // Assemble context
        let ctx = self
            .context_assembler
            .assemble(task_id, &trigger_type, account_id)
            .await?;

        let started_at = std::time::Instant::now();

        // Call LLM
        let llm_response = self
            .llm_provider
            .generate_structured(&ctx.system_prompt, &ctx.user_prompt, task_id)
            .await
            .map_err(|e| AiError::LlmError(e.to_string()))?;

        let duration_ms = i32::try_from(started_at.elapsed().as_millis()).unwrap_or(i32::MAX);

        // Parse response as SuggestionGroup
        let suggestion_group: SuggestionGroup = serde_json::from_str(&llm_response.content)
            .map_err(|e| {
                AiError::LlmError(format!(
                    "Failed to parse SuggestionGroup from LLM response: {e}"
                ))
            })?;

        let ai_suggestion_content = AiSuggestionContent { suggestion_group };

        // Insert suggestion message
        let message_id = AiSuggestionRepository::insert_suggestion_message(
            &self.pool,
            task_id,
            None,
            &ai_suggestion_content,
        )
        .await?;

        // Insert AI context record
        AiContextRepository::insert(
            &self.pool,
            &InsertAiContextParams {
                id: Uuid::now_v7(),
                task_id,
                message_id,
                prompt: format!("{}\n\n{}", ctx.system_prompt, ctx.user_prompt),
                response: llm_response.content,
                model: llm_response.model,
                input_tokens: llm_response.input_tokens,
                output_tokens: llm_response.output_tokens,
                duration_ms,
            },
        )
        .await?;

        // Publish domain event
        self.event_bus.publish(DomainEvent::SuggestionGenerated {
            message_id,
            task_id,
        });

        // Mark event as completed
        AiPipelineRepository::complete_event(&self.pool, event.id).await?;

        tracing::info!(
            event_id = %event.id,
            task_id = %task_id,
            message_id = %message_id,
            "AI pipeline: event completed"
        );

        Ok(())
    }
}
