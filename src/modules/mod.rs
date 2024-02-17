use crate::error::AppError;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{fairing::AdHoc, response, serde::json::Json, Request, Response};

pub mod common;
pub mod role;

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

impl<'r> Responder<'r, 'static> for EmptyResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build().status(Status::NoContent).ok()
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("common stage", |rocket| async {
        rocket
            .mount("/api", [common::routes(), role::routes()].concat())
            .register("/api", catchers![catch_unauthorized, catch_not_found])
    })
}
