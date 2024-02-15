use rocket::serde::json::serde_json;
use rocket::{
    http::{ContentType, Status},
    response::{self, Responder},
    Request, Response,
};
use std::io::Cursor;

#[derive(Debug)]
pub struct AppError {
    status: Status,
    message: String,
}

impl AppError {
    pub const fn not_found(message: String) -> AppError {
        AppError {
            status: Status::NotFound,
            message: message,
        }
    }
    pub fn unauthorized() -> AppError {
        AppError {
            status: Status::Unauthorized,
            message: "Unauthorized".to_owned(),
        }
    }
    pub fn bad_request(message: String) -> AppError {
        AppError {
            status: Status::BadRequest,
            message: message,
        }
    }
    pub fn too_many_requests() -> AppError {
        AppError {
            status: Status::TooManyRequests,
            message: "Too many requests".to_owned(),
        }
    }
    pub fn unknown_host() -> AppError {
        AppError {
            status: Status::BadRequest,
            message: "Unknown host".to_owned(),
        }
    }

    pub fn internal(message: String) -> AppError {
        AppError {
            status: Status::InternalServerError,
            message: message,
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    status: String,
    message: String,
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let body = serde_json::to_string(&ErrorResponse {
            status: self.status.to_string(),
            message: self.message.to_owned(),
        })
        .expect("Failed to serialize error message");

        Response::build()
            .status(self.status)
            .header(ContentType::JSON)
            .sized_body(Some(body.len()), Cursor::new(body))
            .ok()
    }
}
