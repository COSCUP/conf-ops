use std::net::IpAddr;

use lettre::{message::header::ContentType, Message};
use rocket::{
    fairing::AdHoc, http::{Cookie, CookieJar, Status}, response::{self, Responder}, serde::json::Json, time::{Duration, OffsetDateTime}, Request, Response, State
};

use crate::{
    error::AppError, models::{project::Project, user::User, user_email::UserEmail, user_session::UserSession}, modules::{EmptyResponse, EmptyResult, JsonResult}, utils::{jwt, lettre::send_email, rocket::{PrefixUri, UserAgent}}, AppConfig, DbConn
};

#[get("/")]
pub async fn ping() -> &'static str {
    "pong"
}

#[get("/projects")]
pub async fn all_projects(mut db: DbConn) -> JsonResult<Vec<Project>> {
    Ok(Project::all(&mut db).await.map_or(Json(vec![]), Json))
}

#[get("/project")]
pub async fn get_project(project: Project) -> JsonResult<Project> {
    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct LoginReq {
    email: String,
}

#[post("/project/login", data = "<login_req>")]
pub async fn login(
    mut db: DbConn,
    config: &State<AppConfig>,
    host: PrefixUri,
    project: Project,
    login_req: Json<LoginReq>,
) -> EmptyResult {
    match UserEmail::get_user(&mut db, login_req.email.clone()).await {
        Ok(user) => {
            if user.project_id != project.id {
                return Err(AppError::unauthorized());
            }

            let login_token = match jwt::generate_login_token(config, user.id.clone()) {
                Ok(token) => token,
                Err(_) => return Err(AppError::internal("Failed to generate login token".to_owned())),
            };

            let smtp_from = &config.smtp_from;
            let User { name, .. } = user;
            let PrefixUri(prefix_uri) = host;
            let to = login_req.email.clone();

            let message = Message::builder()
                .from(format!("ConfOps <{smtp_from}>").parse().expect("Failed to parse from email address"))
                .to(format!("{name} <{to}>").parse().expect("Failed to parse to email address"))
                .subject("Welcome to ConfOps!")
                .header(ContentType::TEXT_PLAIN)
                .body(format!(
                    "Click here to login: {prefix_uri}/token/{login_token}\nPs. this link is alive in 15 mins."
                ))
                .expect("Failed to build email message");

            let _ = send_email(config, message).await;

            Ok(EmptyResponse)
        }
        Err(_) => Err(AppError::bad_request("No register user".to_owned())),
    }
}

#[derive(Deserialize)]
pub struct TokenReq {
    token: String,
}

#[post("/project/token", data = "<token_req>")]
pub async fn token(
    mut db: DbConn,
    config: &State<AppConfig>,
    cookie_jar: &CookieJar<'_>,
    project: Project,
    user_agent: UserAgent,
    ip: IpAddr,
    token_req: Json<TokenReq>,
) -> EmptyResult {
    let user_id = match jwt::validate_login_token(config, token_req.token.clone()) {
        Ok(token_data) => token_data.claims.user_id,
        Err(err) => return Err(AppError::bad_request(err.to_string())),
    };

    let user = match User::find(&mut db, user_id.clone()).await {
        Ok(user) => user,
        Err(_) => return Err(AppError::bad_request("Invalid token".to_owned())),
    };

    if user.project_id != project.id {
        return Err(AppError::bad_request("Invalid token".to_owned()));
    }

    let session =
        match UserSession::create(&mut db, user.id.clone(), user_agent.0, ip.to_string()).await {
            Ok(session) => session,
            Err(e) => return Err(AppError::internal(e.to_string())),
        };

    let mut cookie = Cookie::new("session_id", session.id.clone());
    cookie.set_http_only(true);
    cookie.set_same_site(rocket::http::SameSite::Lax);
    cookie.set_expires(OffsetDateTime::now_utc() + Duration::days(7));
    cookie_jar.add_private(cookie);

    Ok(EmptyResponse)
}

#[get("/project/me")]
pub async fn get_me(user: User) -> JsonResult<User> {
    Ok(Json(user))
}

#[post("/project/logout")]
pub async fn logout(mut db: DbConn, cookie_jar: &CookieJar<'_>, user_session: UserSession) -> EmptyResult {
    let _ = user_session.expire(&mut db).await;

    if let Some(cookie) = cookie_jar.get_private("session_id") {
        cookie_jar.remove_private(cookie);
    }

    Ok(EmptyResponse)
}

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
        Response::build()
            .status(Status::NoContent)
            .ok()
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("common stage", |rocket| async {
        rocket
            .mount(
                "/api",
                routes![
                    ping,
                    all_projects,
                    get_project,
                    login,
                    token,
                    get_me,
                    logout
                ],
            )
            .register("/api", catchers![catch_unauthorized, catch_not_found])
    })
}
