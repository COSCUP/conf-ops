use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use rocket::State;

use crate::AppConfig;

pub async fn send_email(
    config: &State<AppConfig>,
    message: Message,
) -> Result<(), lettre::transport::smtp::Error> {
    let smtp_host = &config.smtp_host;
    let smtp_port = config.smtp_port;
    let smtp_user = &config.smtp_user;
    let smtp_password = &config.smtp_password;

    let url = format!("smtp://{smtp_user}:{smtp_password}@{smtp_host}:{smtp_port}");

    // Open a remote connection to gmail
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::from_url(&url)
            .unwrap()
            .build();

    // Send the email
    match mailer.send(message).await {
        Ok(_) => return Ok(()),
        Err(e) => panic!("Could not send email: {e:?}"),
    }
}
