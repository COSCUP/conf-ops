use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use rocket::State;

use crate::AppConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginClaims {
    exp: i64,
    iat: i64,
    sub: String,
    pub user_id: String,
}

const LOGIN_TOKEN_SUBJECT: &str = "conf-ops-login";
const LOGIN_TOKEN_ALGORITHM: Algorithm = Algorithm::HS256;

pub fn generate_login_token(
    config: &State<AppConfig>,
    user_id: String,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secret_key = &config.secret_key;
    let claims = LoginClaims {
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + Duration::minutes(15)).timestamp(),
        sub: LOGIN_TOKEN_SUBJECT.to_owned(),
        user_id,
    };

    encode(
        &Header::new(LOGIN_TOKEN_ALGORITHM),
        &claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )
}

pub fn validate_login_token(
    config: &State<AppConfig>,
    token: String,
) -> Result<TokenData<LoginClaims>, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(LOGIN_TOKEN_ALGORITHM);
    validation.sub = Some(LOGIN_TOKEN_SUBJECT.to_owned());

    decode::<LoginClaims>(
        &token,
        &DecodingKey::from_secret(config.secret_key.as_bytes()),
        &validation,
    )
}
