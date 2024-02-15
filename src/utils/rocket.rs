use rocket::{
    http::{uri::Host, Status},
    outcome::Outcome::{Error, Success},
    request::{self, FromRequest},
    Request,
};

use crate::error::AppError;

pub struct PrefixUri(pub String);
const HOST_WHITELIST: &[rocket::http::uri::Host<'_>; 2] = &[
    Host::new(uri!("localhost:8000")),
    Host::new(uri!("ops.coscup.org")),
];
#[rocket::async_trait]
impl<'r> FromRequest<'r> for PrefixUri {
    type Error = AppError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
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
