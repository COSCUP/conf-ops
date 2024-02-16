use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use rocket::State;

use crate::AppConfig;

pub async fn send_email(
    config: &State<AppConfig>,
    message: Message,
) -> Result<(), lettre::transport::smtp::Error> {
    // Open a remote connection to gmail
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::from_url(&config.smtp_url)
            .expect("Failed to create mailer")
            .build();

    // Send the email
    match mailer.send(message).await {
        Ok(_) => return Ok(()),
        Err(e) => panic!("Could not send email: {e:?}"),
    }
}
