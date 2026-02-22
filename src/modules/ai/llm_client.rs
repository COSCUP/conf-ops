use std::time::Duration;

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::models::{
    AiSuggestionContent, Suggestion, SuggestionContextRef, SuggestionDecision, SuggestionGroup,
    TriggerType,
};

// ── Gemini cachedContents API Types ───────────────────────────

/// Error variants specific to cache creation on the Gemini API.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// The HTTP request itself failed (network, timeout, etc.).
    #[error("HTTP request failed: {0}")]
    Http(String),

    /// The API responded with a non-success status code.
    #[error("API error (status {status}): {message}")]
    ApiError {
        /// HTTP status code returned by the API.
        status: u16,
        /// Error message from the API response body.
        message: String,
    },

    /// The response body could not be parsed.
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// A single part in a cached content entry.
#[derive(Debug, Serialize)]
struct CachedContentPart {
    text: String,
}

/// A content element within the cached contents request.
#[derive(Debug, Serialize)]
struct CachedContentItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<CachedContentPart>,
}

/// Request body for `POST /v1beta/cachedContents`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateCachedContentRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    contents: Vec<CachedContentItem>,
    /// Duration string like `"1800s"`.
    ttl: String,
}

/// Response from `POST /v1beta/cachedContents`.
#[derive(Debug, Deserialize)]
struct CreateCachedContentResponse {
    /// The resource name, e.g. `"cachedContents/abc123"`.
    name: String,
}

// ── LLM Error ────────────────────────────────────────────────

/// Errors that can occur when calling an LLM provider.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// The HTTP request itself failed (network, timeout, etc.).
    #[error("HTTP request failed: {0}")]
    Http(String),

    /// The API responded with a non-success status code.
    #[error("API error (status {status}): {message}")]
    ApiError {
        /// HTTP status code returned by the API.
        status: u16,
        /// Error message from the API response body.
        message: String,
    },

    /// The response body could not be parsed.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// The API rate-limited this request.
    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited {
        /// Suggested delay before retrying, in milliseconds.
        retry_after_ms: u64,
    },
}

// ── LLM Response ─────────────────────────────────────────────

/// A successful response from an LLM provider.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// The text content returned by the model.
    pub content: String,
    /// The model identifier that generated the response.
    pub model: String,
    /// Number of tokens consumed in the prompt/input.
    pub input_tokens: i32,
    /// Number of tokens generated in the completion/output.
    pub output_tokens: i32,
}

// ── LLM Provider Trait ───────────────────────────────────────

/// Abstraction over an LLM backend that can generate structured JSON responses.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a structured (JSON) response given system and user prompts.
    ///
    /// The `task_id` is used for tracing and logging purposes.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::Http`] when the HTTP transport fails.
    /// Returns [`LlmError::ApiError`] when the remote API returns a non-success status.
    /// Returns [`LlmError::RateLimited`] when the API indicates a rate-limit condition.
    /// Returns [`LlmError::ParseError`] when the response body cannot be decoded.
    async fn generate_structured(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        task_id: Uuid,
    ) -> Result<LlmResponse, LlmError>;
}

// ── Gemini Serde Types (private) ─────────────────────────────

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    response_mime_type: String,
    temperature: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    cached_content: Option<String>,
    system_instruction: GeminiContent,
    contents: Vec<GeminiContent>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Deserialize)]
struct GeminiPart2 {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiContent2 {
    parts: Vec<GeminiPart2>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent2,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    prompt_token_count: Option<i32>,
    candidates_token_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    model_version: Option<String>,
    usage_metadata: Option<GeminiUsageMetadata>,
}

// ── Gemini Cache Manager ──────────────────────────────────────

/// Manages Gemini Context Caching to reduce token costs for repeated prompts.
///
/// Caches the `cachedContent` name returned by Gemini's `cachedContents` API,
/// keyed by `task_id`. Cached content is used for system prompts and memory
/// context that rarely change between invocations for the same task.
pub struct GeminiCacheManager {
    cache: Cache<Uuid, String>,
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl GeminiCacheManager {
    /// Create a new `GeminiCacheManager` with a 30-minute TTL (max 500 entries).
    pub fn new(api_key: String, model: String) -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(1800))
            .max_capacity(500)
            .build();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            cache,
            client,
            api_key,
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            model,
        }
    }

    /// Get the cached content name for a task, or return `None` if not cached.
    pub async fn get(&self, task_id: Uuid) -> Option<String> {
        self.cache.get(&task_id).await
    }

    /// Invalidate the cached content for a task (e.g. when memory or tools change).
    pub async fn invalidate(&self, task_id: Uuid) {
        self.cache.invalidate(&task_id).await;
    }

    /// Get an existing cached content name or create one on the Gemini API.
    ///
    /// The `system_prompt` and `memory_context` are cached as the content payload,
    /// with a 30-minute TTL on the Gemini side.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` on HTTP or API failure.
    pub async fn get_or_create(
        &self,
        task_id: Uuid,
        system_prompt: &str,
        memory_context: &str,
    ) -> Result<String, CacheError> {
        // Check in-process cache first
        if let Some(name) = self.cache.get(&task_id).await {
            return Ok(name);
        }

        // Create cached content on Gemini API
        let url = format!("{}/cachedContents?key={}", self.base_url, self.api_key);

        let request_body = CreateCachedContentRequest {
            model: format!("models/{}", self.model),
            display_name: Some(format!("task-{task_id}")),
            contents: vec![CachedContentItem {
                role: Some("user".to_string()),
                parts: vec![
                    CachedContentPart {
                        text: system_prompt.to_string(),
                    },
                    CachedContentPart {
                        text: memory_context.to_string(),
                    },
                ],
            }],
            ttl: "1800s".to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| CacheError::Http(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(CacheError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        let parsed: CreateCachedContentResponse = response
            .json()
            .await
            .map_err(|e| CacheError::ParseError(e.to_string()))?;

        // Store in in-process cache
        self.cache.insert(task_id, parsed.name.clone()).await;

        Ok(parsed.name)
    }
}

// ── Gemini Client ─────────────────────────────────────────────

/// An [`LlmProvider`] that calls the Google Gemini `streamGenerateContent` API.
pub struct GeminiClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    cache_manager: GeminiCacheManager,
}

impl GeminiClient {
    /// Create a new `GeminiClient` with a 60-second request timeout.
    ///
    /// Uses the default Gemini v1beta base URL:
    /// `https://generativelanguage.googleapis.com/v1beta`.
    pub fn new(api_key: String, model: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        let cache_manager = GeminiCacheManager::new(api_key.clone(), model.clone());

        Self {
            client,
            api_key,
            model,
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            cache_manager,
        }
    }

    /// Get the cache manager for external invalidation.
    pub fn cache_manager(&self) -> &GeminiCacheManager {
        &self.cache_manager
    }

    /// Build the endpoint URL for `streamGenerateContent`.
    fn endpoint(&self) -> String {
        format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, self.model, self.api_key
        )
    }

    /// Parse the raw [`GeminiResponse`] into an [`LlmResponse`].
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ParseError`] if there are no candidates or parts.
    fn parse_response(&self, raw: GeminiResponse) -> Result<LlmResponse, LlmError> {
        let content = raw
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| {
                LlmError::ParseError("No candidates or parts in Gemini response".to_string())
            })?;

        let model = raw.model_version.unwrap_or_else(|| self.model.clone());

        let (input_tokens, output_tokens) = raw.usage_metadata.map_or((0, 0), |u| {
            (
                u.prompt_token_count.unwrap_or(0),
                u.candidates_token_count.unwrap_or(0),
            )
        });

        Ok(LlmResponse {
            content,
            model,
            input_tokens,
            output_tokens,
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for GeminiClient {
    /// Send a `streamGenerateContent` request to Gemini with retry on transient errors.
    ///
    /// Uses Server-Sent Events (SSE) streaming to reduce first-token latency.
    /// Aggregates all streamed chunks into a single response.
    ///
    /// Retries up to 3 times (4 total attempts) on HTTP 429, 500, and 503 responses
    /// using exponential backoff: 1 s → 2 s → 4 s.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::Http`] on transport failure.
    /// Returns [`LlmError::RateLimited`] on 429 after all retries are exhausted.
    /// Returns [`LlmError::ApiError`] on other non-success status codes.
    /// Returns [`LlmError::ParseError`] if the response body cannot be decoded.
    async fn generate_structured(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        task_id: Uuid,
    ) -> Result<LlmResponse, LlmError> {
        // Try to use Gemini Context Caching for the system prompt
        let cached_content_name = match self
            .cache_manager
            .get_or_create(task_id, system_prompt, "")
            .await
        {
            Ok(name) => {
                tracing::debug!(task_id = %task_id, cached_name = %name, "Using Gemini cached content");
                Some(name)
            }
            Err(e) => {
                tracing::warn!(task_id = %task_id, error = %e, "Gemini cache creation failed, proceeding without cache");
                None
            }
        };

        let body = GeminiRequest {
            cached_content: cached_content_name,
            system_instruction: GeminiContent {
                role: None,
                parts: vec![GeminiPart {
                    text: system_prompt.to_string(),
                }],
            },
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart {
                    text: user_prompt.to_string(),
                }],
            }],
            generation_config: GeminiGenerationConfig {
                response_mime_type: "application/json".to_string(),
                temperature: 0.2,
            },
        };

        let url = self.endpoint();
        let max_retries: u32 = 3;

        let mut last_error: Option<LlmError> = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s
                let delay_secs = 1u64 << (attempt - 1);
                tracing::warn!(
                    task_id = %task_id,
                    attempt = attempt,
                    delay_secs = delay_secs,
                    "Retrying Gemini request after transient error"
                );
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            }

            let response = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| LlmError::Http(e.to_string()))?;

            let status = response.status();

            if status.is_success() {
                // Streaming SSE: read body as text and parse aggregated chunks
                let body_text = response
                    .text()
                    .await
                    .map_err(|e| LlmError::ParseError(e.to_string()))?;

                return self.parse_streaming_response(&body_text);
            }

            let status_u16 = status.as_u16();
            let body_text = response.text().await.unwrap_or_default();

            match status_u16 {
                429 => {
                    // Rate limited — retry with backoff
                    let retry_after_ms = 1000u64 << attempt.min(10);
                    last_error = Some(LlmError::RateLimited { retry_after_ms });
                }
                500 | 503 => {
                    // Transient server error — retry
                    last_error = Some(LlmError::ApiError {
                        status: status_u16,
                        message: body_text,
                    });
                }
                _ => {
                    // Non-retryable error
                    return Err(LlmError::ApiError {
                        status: status_u16,
                        message: body_text,
                    });
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::Http("Max retries exceeded".to_string())))
    }
}

impl GeminiClient {
    /// Parse a streaming SSE response body into a single aggregated [`LlmResponse`].
    ///
    /// Each SSE chunk is a `data: <json>` line containing a `GeminiResponse`.
    /// Text parts from all chunks are concatenated; usage metadata from the last chunk is used.
    fn parse_streaming_response(&self, sse_body: &str) -> Result<LlmResponse, LlmError> {
        let mut full_text = String::new();
        let mut model_version: Option<String> = None;
        let mut input_tokens = 0i32;
        let mut output_tokens = 0i32;

        for line in sse_body.lines() {
            let data = if let Some(stripped) = line.strip_prefix("data: ") {
                stripped.trim()
            } else {
                continue;
            };

            if data.is_empty() || data == "[DONE]" {
                continue;
            }

            let chunk: GeminiResponse = serde_json::from_str(data)
                .map_err(|e| LlmError::ParseError(format!("SSE chunk parse error: {e}")))?;

            // Aggregate text from all candidates
            for candidate in &chunk.candidates {
                for part in &candidate.content.parts {
                    full_text.push_str(&part.text);
                }
            }

            // Use the latest model version and usage metadata
            if chunk.model_version.is_some() {
                model_version = chunk.model_version;
            }
            if let Some(ref usage) = chunk.usage_metadata {
                if let Some(pt) = usage.prompt_token_count {
                    input_tokens = pt;
                }
                if let Some(ct) = usage.candidates_token_count {
                    output_tokens = ct;
                }
            }
        }

        if full_text.is_empty() {
            // Fallback: try parsing the entire body as a single non-streaming response
            let raw: GeminiResponse = serde_json::from_str(sse_body)
                .map_err(|e| LlmError::ParseError(format!("Non-SSE fallback parse error: {e}")))?;
            return self.parse_response(raw);
        }

        Ok(LlmResponse {
            content: full_text,
            model: model_version.unwrap_or_else(|| self.model.clone()),
            input_tokens,
            output_tokens,
        })
    }
}

// ── Mock LLM Provider ─────────────────────────────────────────

/// A test double for [`LlmProvider`] that returns a preset [`AiSuggestionContent`] response.
pub struct MockLlmProvider;

impl MockLlmProvider {
    /// Create a new `MockLlmProvider`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockLlmProvider {
    /// Always returns a fixed JSON-serialized [`AiSuggestionContent`] with one suggestion.
    ///
    /// # Errors
    ///
    /// This implementation never returns an error.
    async fn generate_structured(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _task_id: Uuid,
    ) -> Result<LlmResponse, LlmError> {
        let content = AiSuggestionContent {
            suggestion_group: SuggestionGroup {
                id: Uuid::nil(),
                trigger: TriggerType::ManualRequest,
                suggestions: vec![Suggestion {
                    id: Uuid::nil(),
                    summary: "Send a welcome email to the new contact".to_string(),
                    tool: "email.send".to_string(),
                    parameters: serde_json::json!({
                        "to": "{{profile.email}}",
                        "subject": "Welcome!",
                        "body": "Hello {{profile.name}}, welcome to the project."
                    }),
                    reasoning:
                        "A new contact was added; sending a welcome email is a common first step."
                            .to_string(),
                    context_used: vec![SuggestionContextRef {
                        scope_type: "task".to_string(),
                        memory_id: Uuid::nil(),
                        content: "Task context: new contact onboarding".to_string(),
                    }],
                    decision: SuggestionDecision::Pending,
                    decided_by: None,
                    decided_at: None,
                    modified_parameters: None,
                    execution_result: None,
                }],
                created_at: chrono::DateTime::from_timestamp(1_700_000_000, 0)
                    .expect("valid timestamp"),
            },
        };

        let json =
            serde_json::to_string(&content).map_err(|e| LlmError::ParseError(e.to_string()))?;

        Ok(LlmResponse {
            content: json,
            model: "mock-llm".to_string(),
            input_tokens: 0,
            output_tokens: 0,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Request serialization ─────────────────────────────────

    #[test]
    fn gemini_request_serializes_correctly() {
        let req = GeminiRequest {
            cached_content: None,
            system_instruction: GeminiContent {
                role: None,
                parts: vec![GeminiPart {
                    text: "You are a helpful assistant.".to_string(),
                }],
            },
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart {
                    text: "What is 2+2?".to_string(),
                }],
            }],
            generation_config: GeminiGenerationConfig {
                response_mime_type: "application/json".to_string(),
                temperature: 0.2,
            },
        };

        let value = serde_json::to_value(&req).expect("should serialize");

        // systemInstruction (camelCase) has no `role` field (skipped)
        assert!(value["systemInstruction"].get("role").is_none());
        assert_eq!(
            value["systemInstruction"]["parts"][0]["text"],
            "You are a helpful assistant."
        );

        // contents[0] has role = "user"
        assert_eq!(value["contents"][0]["role"], "user");
        assert_eq!(value["contents"][0]["parts"][0]["text"], "What is 2+2?");

        // generationConfig uses camelCase
        assert_eq!(
            value["generationConfig"]["responseMimeType"],
            "application/json"
        );
        let temperature = value["generationConfig"]["temperature"]
            .as_f64()
            .expect("temperature is f64");
        // f32 0.2 serialized through JSON may not match f64 0.2 exactly; use 1e-6 tolerance
        assert!((temperature - 0.2_f64).abs() < 1e-6);
    }

    // ── Response parsing ──────────────────────────────────────

    #[test]
    fn gemini_response_parses_correctly() {
        let client = GeminiClient::new("key".to_string(), "gemini-2.0-flash".to_string());

        let raw = GeminiResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContent2 {
                    parts: vec![GeminiPart2 {
                        text: r#"{"hello":"world"}"#.to_string(),
                    }],
                },
            }],
            model_version: Some("gemini-2.0-flash-001".to_string()),
            usage_metadata: Some(GeminiUsageMetadata {
                prompt_token_count: Some(42),
                candidates_token_count: Some(17),
            }),
        };

        let resp = client.parse_response(raw).expect("should parse");
        assert_eq!(resp.content, r#"{"hello":"world"}"#);
        assert_eq!(resp.model, "gemini-2.0-flash-001");
        assert_eq!(resp.input_tokens, 42);
        assert_eq!(resp.output_tokens, 17);
    }

    #[test]
    fn gemini_response_parse_error_on_empty_candidates() {
        let client = GeminiClient::new("key".to_string(), "gemini-2.0-flash".to_string());

        let raw = GeminiResponse {
            candidates: vec![],
            model_version: None,
            usage_metadata: None,
        };

        let err = client.parse_response(raw).unwrap_err();
        assert!(matches!(err, LlmError::ParseError(_)));
    }

    #[test]
    fn gemini_response_falls_back_to_client_model_when_version_missing() {
        let client = GeminiClient::new("key".to_string(), "gemini-2.0-flash-fallback".to_string());

        let raw = GeminiResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContent2 {
                    parts: vec![GeminiPart2 {
                        text: "{}".to_string(),
                    }],
                },
            }],
            model_version: None,
            usage_metadata: None,
        };

        let resp = client.parse_response(raw).expect("should parse");
        assert_eq!(resp.model, "gemini-2.0-flash-fallback");
        assert_eq!(resp.input_tokens, 0);
        assert_eq!(resp.output_tokens, 0);
    }

    // ── Streaming response parsing ─────────────────────────────

    #[test]
    fn parse_streaming_sse_response() {
        let client = GeminiClient::new("key".to_string(), "gemini-2.0-flash".to_string());

        let sse_body = r#"data: {"candidates":[{"content":{"parts":[{"text":"{\"id\":"}]}}],"modelVersion":"gemini-2.0-flash-001","usageMetadata":{"promptTokenCount":42,"candidatesTokenCount":5}}
data: {"candidates":[{"content":{"parts":[{"text":"\"abc\"}"}]}}],"usageMetadata":{"promptTokenCount":42,"candidatesTokenCount":12}}
data: [DONE]
"#;

        let resp = client
            .parse_streaming_response(sse_body)
            .expect("should parse SSE");
        assert_eq!(resp.content, r#"{"id":"abc"}"#);
        assert_eq!(resp.model, "gemini-2.0-flash-001");
        assert_eq!(resp.input_tokens, 42);
        assert_eq!(resp.output_tokens, 12);
    }

    #[test]
    fn parse_streaming_response_fallback_to_non_sse() {
        let client = GeminiClient::new("key".to_string(), "gemini-2.0-flash".to_string());

        // Non-SSE body (single JSON response)
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"hello"}]}}],"modelVersion":"gemini-2.0-flash-001","usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":1}}"#;

        let resp = client
            .parse_streaming_response(body)
            .expect("should fallback parse");
        assert_eq!(resp.content, "hello");
    }

    // ── Cache manager ────────────────────────────────────────

    #[tokio::test]
    async fn cache_manager_get_or_create_uses_in_process_cache() {
        let mgr = GeminiCacheManager::new("test-key".to_string(), "test-model".to_string());
        let task_id = Uuid::new_v4();

        assert!(mgr.get(task_id).await.is_none());

        // Manually insert to simulate a previous get_or_create
        mgr.cache
            .insert(task_id, "cachedContents/abc123".to_string())
            .await;
        let result = mgr.get_or_create(task_id, "system", "memory").await;
        assert_eq!(result.unwrap(), "cachedContents/abc123");

        mgr.invalidate(task_id).await;
        assert!(mgr.get(task_id).await.is_none());
    }

    // ── Mock provider ─────────────────────────────────────────

    #[tokio::test]
    async fn mock_provider_returns_valid_suggestion_json() {
        let provider = MockLlmProvider::new();
        let task_id = Uuid::nil();

        let resp = provider
            .generate_structured("system", "user", task_id)
            .await
            .expect("mock should not fail");

        assert_eq!(resp.model, "mock-llm");
        assert_eq!(resp.input_tokens, 0);
        assert_eq!(resp.output_tokens, 0);

        // The content must be valid JSON that deserializes into AiSuggestionContent
        let content: AiSuggestionContent =
            serde_json::from_str(&resp.content).expect("mock content should be valid JSON");

        assert_eq!(content.suggestion_group.suggestions.len(), 1);
        let suggestion = &content.suggestion_group.suggestions[0];
        assert_eq!(suggestion.decision, SuggestionDecision::Pending);
        assert!(!suggestion.tool.is_empty());
        assert!(!suggestion.summary.is_empty());
    }

    #[tokio::test]
    async fn mock_provider_default_constructor() {
        let provider = MockLlmProvider::default();
        let resp = provider
            .generate_structured("s", "u", Uuid::nil())
            .await
            .expect("default mock should not fail");
        assert_eq!(resp.model, "mock-llm");
    }

    #[test]
    fn cached_content_request_serializes_correctly() {
        let req = CreateCachedContentRequest {
            model: "models/gemini-2.5-flash".to_string(),
            display_name: Some("task-abc".to_string()),
            contents: vec![CachedContentItem {
                role: Some("user".to_string()),
                parts: vec![CachedContentPart {
                    text: "system prompt".to_string(),
                }],
            }],
            ttl: "1800s".to_string(),
        };
        let value = serde_json::to_value(&req).expect("should serialize");
        assert_eq!(value["model"], "models/gemini-2.5-flash");
        assert_eq!(value["displayName"], "task-abc");
        assert_eq!(value["ttl"], "1800s");
        assert_eq!(value["contents"][0]["role"], "user");
        assert_eq!(value["contents"][0]["parts"][0]["text"], "system prompt");
    }

    #[test]
    fn cached_content_response_deserializes_correctly() {
        let json_str = r#"{"name":"cachedContents/abc123"}"#;
        let resp: CreateCachedContentResponse =
            serde_json::from_str(json_str).expect("should deserialize");
        assert_eq!(resp.name, "cachedContents/abc123");
    }
}
