use std::{num::NonZeroU32, time::Duration};

use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use rocket::{
    http::{uri::Host, Status},
    outcome::Outcome::{Error, Success},
    request::{self, FromRequest},
    Request,
};

use crate::error::AppError;

pub struct PrefixUri(pub String);
const HOST_WHITELIST: &[rocket::http::uri::Host<'_>; 2] = &[
    Host::new(uri!("127.0.0.1:8000")),
    Host::new(uri!("ops.coscup.org")),
];
#[rocket::async_trait]
impl<'r> FromRequest<'r> for PrefixUri {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let origin = request.headers().get_one("Origin");

        if let Some(origin) = origin {
            return Success(PrefixUri(origin.to_owned()));
        }

        let uri = request
            .host()
            .and_then(|host| host.to_absolute("http", HOST_WHITELIST));

        match uri {
            Some(host) => Success(PrefixUri(host.to_string())),
            None => Error((Status::BadRequest, AppError::unknown_host())),
        }
    }
}

pub struct UserAgent(pub String);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserAgent {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, AppError> {
        let user_agent = request.headers().get_one("User-Agent").unwrap_or("Unknown");
        Success(UserAgent(user_agent.to_owned()))
    }
}

pub struct EmailRateLimiter(DefaultKeyedRateLimiter<String>);
impl EmailRateLimiter {
    pub fn new() -> Self {
        let quota = Quota::with_period(Duration::from_secs(60 * 5)).unwrap();
        let limiter = RateLimiter::keyed(quota);
        EmailRateLimiter(limiter)
    }

    pub fn check_key(&self, email: &String) -> Result<(), AppError> {
        match self.0.check_key(email) {
            Ok(_) => Ok(()),
            Err(_) => Err(AppError::too_many_requests()),
        }
    }
}

pub struct VerifyEmailOrTokenRateLimiter(DefaultKeyedRateLimiter<String>);
impl VerifyEmailOrTokenRateLimiter {
    pub fn new() -> Self {
        let limiter = RateLimiter::keyed(Quota::per_minute(NonZeroU32::new(5).unwrap()));
        VerifyEmailOrTokenRateLimiter(limiter)
    }

    pub fn check_key(&self, ip: &String) -> Result<(), AppError> {
        match self.0.check_key(ip) {
            Ok(_) => Ok(()),
            Err(_) => Err(AppError::too_many_requests()),
        }
    }
}
