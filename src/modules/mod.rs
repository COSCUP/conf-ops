use crate::error::AppError;
use crate::models::project::Project;
use crate::models::user::User;
use crate::models::user_session::UserSession;
use crate::DbConn;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{fairing::AdHoc, response, serde::json::Json, Request, Response};

pub mod common;
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

impl<'r> Responder<'r, 'static> for EmptyResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build().status(Status::NoContent).ok()
    }
}

pub struct AuthGuard {
    pub project: Project,
    pub user: User,
    pub user_session: UserSession,
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for AuthGuard {
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "type", content = "todo")]
pub enum EnabledFeature {
    ManagerRole(usize),
    Ticket(usize),
    ManagerTicket(usize),
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Common Stage", |rocket| async {
        rocket
            .mount("/api", common::routes())
            .mount(
                "/api/project",
                [role::routes(), ticket::api::routes()].concat(),
            )
            .register("/api", catchers![catch_unauthorized, catch_not_found])
    })
}
