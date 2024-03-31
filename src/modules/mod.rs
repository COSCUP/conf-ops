use crate::error::AppError;
use crate::utils::rocket::{EmailRateLimiter, VerifyEmailOrTokenRateLimiter};
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{fairing::AdHoc, response, serde::json::Json, Request, Response};

pub mod common;
pub mod guard;
pub mod role;
pub mod ticket;

type ApiResult<T> = Result<T, crate::error::AppError>;
pub type JsonResult<T> = ApiResult<Json<T>>;
pub struct EmptyResponse;
pub type EmptyResult = ApiResult<EmptyResponse>;

#[catch(401)]
fn catch_unauthorized() -> AppError {
    AppError::unauthorized()
}

#[catch(404)]
fn catch_not_found() -> AppError {
    AppError::not_found("Resource not found".to_owned())
}

#[catch(429)]
fn catch_too_many_requests() -> AppError {
    AppError::too_many_requests()
}

#[catch(500)]
fn catch_internal() -> AppError {
    AppError::internal(
        "The server encountered an internal error while processing this request.".to_owned(),
    )
}

impl<'r> Responder<'r, 'static> for EmptyResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build().status(Status::NoContent).ok()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "type", content = "todo")]
pub enum EnabledFeature {
    RoleManage(usize),
    Ticket(usize),
    TicketManage(usize),
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Common Stage", |rocket| async {
        rocket
            .manage(EmailRateLimiter::new())
            .manage(VerifyEmailOrTokenRateLimiter::new())
            .mount("/api", common::routes())
            .mount(
                "/api/project",
                [role::routes(), ticket::api::routes()].concat(),
            )
            .register(
                "/api",
                catchers![
                    catch_unauthorized,
                    catch_not_found,
                    catch_internal,
                    catch_too_many_requests
                ],
            )
    })
}
