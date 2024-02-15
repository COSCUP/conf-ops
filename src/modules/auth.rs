use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use lettre::message::header::ContentType;
use lettre::Message;
use rocket::State;

use crate::utils::lettre::send_email;
use crate::utils::rocket::PrefixUri;
use crate::{models::user::User, AppConfig};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginClaims {
    exp: i64,
    sub: String,
    pub user_id: String,
}

pub async fn send_login_email(
    config: &State<AppConfig>,
    host: PrefixUri,
    to: String,
    user: User,
) -> Result<(), lettre::transport::smtp::Error> {
    let smtp_from = &config.smtp_from;
    let secret_key = &config.secret_key;
    let User { name, .. } = user;

    let claims = LoginClaims {
        exp: (Utc::now() + Duration::minutes(15)).timestamp(),
        sub: "conf-ops-login".to_owned(),
        user_id: user.id,
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )
    .unwrap();

    let PrefixUri(prefix_uri) = host;

    let message = Message::builder()
        .from(format!("ConfOps <{smtp_from}>").parse().unwrap())
        .to(format!("{name} <{to}>").parse().unwrap())
        .subject("Welcome to ConfOps!")
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Click here to login: {prefix_uri}/token/{token}\nPs. this link is alive in 15 mins."
        ))
        .unwrap();

    send_email(config, message).await
}

pub fn validate_token(
    config: &State<AppConfig>,
    token: String,
) -> Result<TokenData<LoginClaims>, jsonwebtoken::errors::Error> {
    decode::<LoginClaims>(
        &token,
        &DecodingKey::from_secret(config.secret_key.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
}
