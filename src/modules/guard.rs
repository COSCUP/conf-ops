use std::net::IpAddr;

use rocket::request::FromRequest;
use rocket::State;
use rocket::Data;
use rocket::Request;
use rocket::data::FromData;
use rocket::serde::json::Json;

use crate::error::AppError;
use crate::models::project::Project;
use crate::models::user::User;
use crate::models::user_session::UserSession;
use crate::utils::rocket::EmailRateLimiter;
use crate::utils::rocket::VerifyEmailOrTokenRateLimiter;
use crate::DbConn;
use crate::modules::common::LoginReq;

pub struct LoginReqGuard (pub Json<LoginReq>);

#[rocket::async_trait]
impl<'r> FromData<'r> for LoginReqGuard {
    type Error = AppError;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> rocket::data::Outcome<'r, Self, Self::Error> {
        let login_req = match Json::<LoginReq>::from_data(req, data).await {
            rocket::outcome::Outcome::Success(login_req) => login_req,
            rocket::outcome::Outcome::Forward(s) => return rocket::outcome::Outcome::Forward(s),
            rocket::outcome::Outcome::Error((status, error)) => {
                return rocket::outcome::Outcome::Error((
                    status,
                    AppError::bad_request(error.to_string())
                ));
            }
        };

        let rate_limiter = match req.guard::<&State<EmailRateLimiter>>().await {
            rocket::outcome::Outcome::Success(rate_limiter) => rate_limiter,
            rocket::outcome::Outcome::Forward(s) => return rocket::outcome::Outcome::Error((s, AppError::internal("Failed to get rate limiter".to_owned()))),
            rocket::outcome::Outcome::Error((status, _error)) => {
                return rocket::outcome::Outcome::Error((status, AppError::internal("Failed to get rate limiter".to_owned())));
            }
        };

        match rate_limiter.check_key(&login_req.email) {
            Ok(_) => rocket::outcome::Outcome::Success(LoginReqGuard(login_req)),
            Err(err) => rocket::outcome::Outcome::Error((rocket::http::Status::TooManyRequests, err))
        }
    }
}

pub struct VerifyEmailOrTokenGuard(pub IpAddr);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for VerifyEmailOrTokenGuard {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let ip = match req.guard::<IpAddr>().await {
            rocket::outcome::Outcome::Success(ip) => ip,
            rocket::outcome::Outcome::Forward(s) => return rocket::outcome::Outcome::Forward(s),
            rocket::outcome::Outcome::Error((status, error)) => {
                return rocket::outcome::Outcome::Error((status, AppError::internal(error.to_string())));
            }
        };

        let rate_limiter = match req.guard::<&State<VerifyEmailOrTokenRateLimiter>>().await {
            rocket::outcome::Outcome::Success(rate_limiter) => rate_limiter,
            rocket::outcome::Outcome::Forward(s) => return rocket::outcome::Outcome::Error((s, AppError::internal("Failed to get rate limiter".to_owned()))),
            rocket::outcome::Outcome::Error((status, _error)) => {
                return rocket::outcome::Outcome::Error((status, AppError::internal("Failed to get rate limiter".to_owned())));
            }
        };

        match rate_limiter.check_key(&ip.to_string()) {
            Ok(_) => rocket::outcome::Outcome::Success(VerifyEmailOrTokenGuard(ip)),
            Err(err) => rocket::outcome::Outcome::Error((rocket::http::Status::TooManyRequests, err))
        }
    }
}

pub struct AuthGuard {
    pub project: Project,
    pub user: User,
    pub user_session: UserSession,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthGuard {
    type Error = AppError;

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let mut db = match request.guard::<DbConn>().await {
            rocket::outcome::Outcome::Success(db) => db,
            rocket::outcome::Outcome::Error(error) => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::InternalServerError,
                    AppError::internal(
                        error
                            .1
                            .map_or("Unknown database problem".to_owned(), |err| err.to_string()),
                    ),
                ))
            }
            rocket::outcome::Outcome::Forward(s) => return rocket::outcome::Outcome::Forward(s),
        };

        let session_cookie = match request.cookies().get_private("session_id") {
            Some(cookie) => cookie,
            None => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::Unauthorized,
                    AppError::unauthorized(),
                ))
            }
        };

        let session_id = match session_cookie.value().parse() {
            Ok(session_id) => session_id,
            _ => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::Unauthorized,
                    AppError::unauthorized(),
                ))
            }
        };

        let auth = match UserSession::auth(&mut db, session_id).await {
            Ok(user) => user,
            Err(_) => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::Unauthorized,
                    AppError::unauthorized(),
                ))
            }
        };

        rocket::request::Outcome::Success(auth)
    }
}
