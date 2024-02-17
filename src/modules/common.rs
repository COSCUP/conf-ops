use std::net::IpAddr;

use lettre::{message::header::ContentType, Message};
use rocket::{
    http::{Cookie, CookieJar},
    serde::json::Json,
    time::{Duration, OffsetDateTime},
    Route, State,
};

use crate::{
    error::AppError,
    models::{project::Project, user::User, user_email::UserEmail, user_session::UserSession},
    modules::{EmptyResponse, EmptyResult, JsonResult},
    utils::{
        jwt::{self, LoginClaims},
        lettre::send_email,
        rocket::{PrefixUri, UserAgent},
    },
    AppConfig, DbConn,
};

use super::AuthGuard;

#[get("/")]
pub async fn ping() -> &'static str {
    "pong"
}

#[get("/projects")]
pub async fn all_projects(mut conn: DbConn) -> JsonResult<Vec<Project>> {
    Ok(Project::all(&mut conn).await.map_or(Json(vec![]), Json))
}

#[get("/project")]
pub async fn get_project(auth: AuthGuard) -> JsonResult<Project> {
    let AuthGuard { project, .. } = auth;
    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct LoginReq {
    project_id: String,
    email: String,
}

#[post("/project/login", data = "<login_req>")]
pub async fn login(
    mut conn: DbConn,
    config: &State<AppConfig>,
    host: PrefixUri,
    login_req: Json<LoginReq>,
) -> EmptyResult {
    let user = UserEmail::get_user(
        &mut conn,
        login_req.project_id.clone(),
        login_req.email.clone(),
    )
    .await
    .map_err(|err| AppError::bad_request(err.to_string()))?;

    let login_token = jwt::generate_login_token(config, user.project_id.clone(), user.id.clone())
        .map_err(|err| AppError::internal(err.to_string()))?;

    let email_from = &config.email_from;
    let User { name, .. } = user;
    let PrefixUri(prefix_uri) = host;
    let to = login_req.email.clone();

    let message = Message::builder()
        .from(format!("ConfOps <{email_from}>").parse().expect("Failed to parse from email address"))
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

#[derive(Deserialize)]
pub struct TokenReq {
    token: String,
}

#[post("/project/token", data = "<token_req>")]
pub async fn token(
    mut conn: DbConn,
    config: &State<AppConfig>,
    cookie_jar: &CookieJar<'_>,
    user_agent: UserAgent,
    ip: IpAddr,
    token_req: Json<TokenReq>,
) -> EmptyResult {
    let LoginClaims {
        project_id,
        user_id,
        ..
    } = jwt::validate_login_token(config, token_req.token.clone())
        .map_err(|err| AppError::bad_request(err.to_string()))?
        .claims;

    let user = User::find(&mut conn, user_id.clone())
        .await
        .map_err(|err| AppError::bad_request(err.to_string()))?;

    if user.project_id != project_id {
        return Err(AppError::bad_request("Invalid token".to_owned()));
    }

    let session = UserSession::create(&mut conn, user.id.clone(), user_agent.0, ip.to_string())
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let mut cookie = Cookie::new("session_id", session.id.clone());
    cookie.set_http_only(true);
    cookie.set_same_site(rocket::http::SameSite::Lax);
    cookie.set_expires(OffsetDateTime::now_utc() + Duration::days(7));
    cookie_jar.add_private(cookie);

    Ok(EmptyResponse)
}

#[get("/project/me")]
pub async fn get_me(auth: AuthGuard) -> JsonResult<User> {
    let AuthGuard { user, .. } = auth;
    Ok(Json(user))
}

#[post("/project/logout")]
pub async fn logout(mut conn: DbConn, cookie_jar: &CookieJar<'_>, auth: AuthGuard) -> EmptyResult {
    let AuthGuard { user_session, .. } = auth;
    let _ = user_session.expire(&mut conn).await;

    if let Some(cookie) = cookie_jar.get_private("session_id") {
        cookie_jar.remove_private(cookie);
    }

    Ok(EmptyResponse)
}

pub fn routes() -> Vec<Route> {
    routes![
        ping,
        all_projects,
        get_project,
        login,
        token,
        get_me,
        logout
    ]
}
