use rocket::serde::json::{serde_json, Value};
use rocket::{
    http::{ContentType, Status},
    response::{self, Responder},
    Request, Response,
};
use std::io::Cursor;

use crate::utils::i18n::I18n;

#[derive(Debug, Serialize, Clone)]
pub struct AppError {
    status: Status,
    message: String,
    fields: Option<serde_json::Map<String, Value>>,
}

impl AppError {
    pub const fn not_found(message: String) -> AppError {
        AppError {
            status: Status::NotFound,
            message: message,
            fields: None,
        }
    }
    pub fn unauthorized(i18n: I18n) -> AppError {
        AppError {
            status: Status::Unauthorized,
            message: i18n.t("error.unauthorized"),
            fields: None,
        }
    }
    pub fn bad_request(message: String) -> AppError {
        AppError {
            status: Status::BadRequest,
            message,
            fields: None,
        }
    }
    pub fn bad_request_with_fields(i18n: I18n, fields: serde_json::Map<String, Value>) -> AppError {
        AppError {
            status: Status::BadRequest,
            message: i18n.t("error.wrong_fields"),
            fields: Some(fields),
        }
    }
    pub fn forbidden(message: String) -> AppError {
        AppError {
            status: Status::Forbidden,
            message: message,
            fields: None,
        }
    }
    pub fn too_many_requests(i18n: I18n) -> AppError {
        AppError {
            status: Status::TooManyRequests,
            message: i18n.t("error.too_many_requests"),
            fields: None,
        }
    }
    pub fn unknown_host(i18n: I18n) -> AppError {
        AppError {
            status: Status::BadRequest,
            message: i18n.t("error.unknown_host"),
            fields: None,
        }
    }

    pub fn internal(message: String) -> AppError {
        AppError {
            status: Status::InternalServerError,
            message: message,
            fields: None,
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    status: String,
    message: String,
    fields: Option<serde_json::Map<String, Value>>,
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let body = serde_json::to_string(&ErrorResponse {
            status: self.status.to_string(),
            message: self.message.to_owned(),
            fields: self.fields,
        })
        .expect("Failed to serialize error message");

        Response::build()
            .status(self.status)
            .header(ContentType::JSON)
            .sized_body(Some(body.len()), Cursor::new(body))
            .ok()
    }
}
