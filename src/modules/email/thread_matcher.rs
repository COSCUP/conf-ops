use sqlx::PgPool;

use super::error::EmailError;
use super::mime_parser::{normalize_subject, ParsedEmail};
use super::models::EmailThread;
use super::repository::{EmailMessageRepository, EmailThreadRepository};

#[derive(Debug)]
pub enum ThreadMatchResult {
    ExactMatch(EmailThread),
    HeuristicMatch(EmailThread),
    NoMatch,
}

/// Match an inbound email to an existing thread using a three-stage algorithm.
///
/// 1. Exact match via `In-Reply-To` header -> `email_messages.message_id`
/// 2. Exact match via `References` header -> `email_threads.message_ids` (GIN)
/// 3. Heuristic match via normalized subject + sender address
///
/// # Errors
///
/// Returns `EmailError::Database` on database failure.
pub async fn match_thread(
    pool: &PgPool,
    parsed: &ParsedEmail,
) -> Result<ThreadMatchResult, EmailError> {
    // Stage 1: In-Reply-To exact match
    if let Some(ref in_reply_to) = parsed.in_reply_to {
        if let Some(existing_msg) =
            EmailMessageRepository::find_by_message_id(pool, in_reply_to).await?
        {
            let thread = EmailThreadRepository::get_by_id(pool, existing_msg.thread_id).await?;
            return Ok(ThreadMatchResult::ExactMatch(thread));
        }
    }

    // Stage 2: References GIN match
    for ref_id in &parsed.references {
        if let Some(thread) =
            EmailThreadRepository::find_by_message_ids_contains(pool, ref_id).await?
        {
            return Ok(ThreadMatchResult::ExactMatch(thread));
        }
    }

    // Stage 3: Heuristic - normalized subject + sender
    let normalized = normalize_subject(&parsed.subject);
    if !normalized.is_empty() {
        if let Some(thread) = EmailThreadRepository::find_by_subject_and_sender(
            pool,
            &normalized,
            &parsed.from_address,
        )
        .await?
        {
            return Ok(ThreadMatchResult::HeuristicMatch(thread));
        }
    }

    Ok(ThreadMatchResult::NoMatch)
}
