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

#[async_trait]
pub trait EmailService: Send + Sync {
    /// Send an email.
    ///
    /// # Errors
    ///
    /// Returns `EmailError` if sending fails.
    async fn send(&self, to: &str, subject: &str, html_body: &str) -> Result<(), EmailError>;
}
