pub mod error;
pub mod inbound;
pub mod mime_parser;
pub mod models;
pub mod repository;
pub mod sender_resolver;
pub mod service;
pub mod smtp;
pub mod thread_matcher;

use async_trait::async_trait;

use crate::modules::email::error::EmailError;

/// Email headers for threading (Message-ID, In-Reply-To, References).
#[derive(Debug, Clone, Default)]
pub struct EmailHeaders {
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
}

#[async_trait]
pub trait EmailService: Send + Sync {
    /// Send an email.
    ///
    /// # Errors
    ///
    /// Returns `EmailError` if sending fails.
    async fn send(&self, to: &str, subject: &str, html_body: &str) -> Result<(), EmailError>;

    /// Send an email with threading headers (Message-ID, In-Reply-To, References).
    ///
    /// # Errors
    ///
    /// Returns `EmailError` if sending fails.
    async fn send_with_headers(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        _headers: &EmailHeaders,
    ) -> Result<(), EmailError> {
        // Default implementation delegates to send() for backwards compatibility
        self.send(to, subject, html_body).await
    }
}
