use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use super::EmailService;
use crate::config::AppConfig;

pub struct SmtpEmailService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpEmailService {
    /// Create a new SMTP email service from app config.
    ///
    /// # Errors
    ///
    /// Returns an error string if the transport cannot be created.
    pub fn new(config: &AppConfig) -> Result<Self, String> {
        let from: Mailbox = config
            .smtp_from
            .parse()
            .map_err(|e| format!("Invalid SMTP_FROM address: {e}"))?;

        let mut builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
                .port(config.smtp_port);

        if let (Some(username), Some(password)) = (&config.smtp_username, &config.smtp_password) {
            builder = builder.credentials(Credentials::new(username.clone(), password.clone()));
        }

        let transport = builder.build();

        Ok(Self { transport, from })
    }
}

#[async_trait]
impl EmailService for SmtpEmailService {
    async fn send(&self, to: &str, subject: &str, html_body: &str) -> Result<(), String> {
        let to_mailbox: Mailbox = to
            .parse()
            .map_err(|e| format!("Invalid recipient address: {e}"))?;

        let email = Message::builder()
            .from(self.from.clone())
            .to(to_mailbox)
            .subject(subject)
            .header(lettre::message::header::ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| format!("Failed to build email: {e}"))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| format!("Failed to send email: {e}"))?;

        Ok(())
    }
}
