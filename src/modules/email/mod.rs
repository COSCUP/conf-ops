pub mod smtp;

use async_trait::async_trait;

#[async_trait]
pub trait EmailService: Send + Sync {
    /// Send an email.
    ///
    /// # Errors
    ///
    /// Returns an error string if sending fails.
    async fn send(&self, to: &str, subject: &str, html_body: &str) -> Result<(), String>;
}
