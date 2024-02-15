use rocket::serde::json::Json;

pub mod auth;
pub mod base;

type ApiResult<T> = Result<T, crate::error::AppError>;
pub type JsonResult<T> = ApiResult<Json<T>>;
pub struct EmptyResponse;
pub type EmptyResult = ApiResult<EmptyResponse>;
