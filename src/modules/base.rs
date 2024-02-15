use std::net::IpAddr;

use rocket::{
    fairing::AdHoc, http::{Cookie, CookieJar, Status}, response::{self, Responder}, serde::json::Json, time::{Duration, OffsetDateTime}, Request, Response, State
};

use crate::{
    error::AppError, models::{project::Project, user::User, user_email::UserEmail, user_session::UserSession}, modules::{auth, JsonResult, EmptyResult, EmptyResponse}, utils::rocket::{PrefixUri, UserAgent}, AppConfig, DbConn
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
            auth::send_login_email(config, host, login_req.email.clone(), user)
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
            Ok(EmptyResponse)
        }
        Err(_) => Err(AppError::unauthorized()),
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
    let user_id = match auth::validate_token(config, token_req.token.clone()) {
        Ok(token_data) => token_data.claims.user_id,
        Err(_) => return Err(AppError::unauthorized()),
    };

    let user = match User::get(&mut db, user_id.clone()).await {
        Ok(user) => user,
        Err(_) => return Err(AppError::unauthorized()),
    };

    if user.project_id != project.id {
        return Err(AppError::unauthorized());
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
    AdHoc::on_ignite("base stage", |rocket| async {
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
