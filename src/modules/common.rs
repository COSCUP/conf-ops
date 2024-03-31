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

use super::{
    guard::{AuthGuard, LoginReqGuard, VerifyEmailOrTokenGuard},
    role, ticket, EnabledFeature,
};

#[get("/")]
async fn ping() -> &'static str {
    "pong"
}

#[get("/projects")]
async fn all_projects(mut conn: DbConn) -> JsonResult<Vec<Project>> {
    Ok(Project::all(&mut conn).await.map_or(Json(vec![]), Json))
}

#[get("/projects/<project_id>")]
async fn get_project_by_id(mut conn: DbConn, project_id: String) -> JsonResult<Project> {
    Ok(Project::find(&mut conn, project_id)
        .await
        .map(Json)
        .map_err(|err| AppError::not_found(err.to_string()))?)
}

#[get("/project")]
async fn get_auth_project(auth: AuthGuard) -> JsonResult<Project> {
    let AuthGuard { project, .. } = auth;
    Ok(Json(project))
}

#[derive(Deserialize)]
pub struct LoginReq {
    project_id: String,
    pub email: String,
}

#[post("/project/login", data = "<login_req>")]
async fn login(
    mut conn: DbConn,
    config: &State<AppConfig>,
    _ip: VerifyEmailOrTokenGuard,
    host: PrefixUri,
    login_req: LoginReqGuard,
) -> EmptyResult {
    let user = UserEmail::get_user(
        &mut conn,
        login_req.0.project_id.clone(),
        login_req.0.email.clone(),
    )
    .await
    .map_err(|err| AppError::bad_request(err.to_string()))?;

    let login_token = jwt::generate_login_token(config, user.project_id.clone(), user.id.clone())
        .map_err(|err| AppError::internal(err.to_string()))?;

    let email_from = &config.email_from;
    let User { name, .. } = user;
    let PrefixUri(prefix_uri) = host;
    let to = login_req.0.email.clone();

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
struct TokenReq {
    token: String,
}

#[post("/project/token", data = "<token_req>")]
async fn verify_token(
    mut conn: DbConn,
    config: &State<AppConfig>,
    cookie_jar: &CookieJar<'_>,
    user_agent: UserAgent,
    ip: VerifyEmailOrTokenGuard,
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

    let session = UserSession::create(&mut conn, user.id.clone(), user_agent.0, ip.0.to_string())
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
async fn get_me(auth: AuthGuard) -> JsonResult<User> {
    let AuthGuard { user, .. } = auth;
    Ok(Json(user))
}

#[post("/project/logout")]
async fn logout(mut conn: DbConn, cookie_jar: &CookieJar<'_>, auth: AuthGuard) -> EmptyResult {
    let AuthGuard { user_session, .. } = auth;
    let _ = user_session.expire(&mut conn).await;

    if let Some(cookie) = cookie_jar.get_private("session_id") {
        cookie_jar.remove_private(cookie);
    }

    Ok(EmptyResponse)
}

#[get("/project/features")]
async fn get_features_by_user(
    mut conn: DbConn,
    auth: AuthGuard,
) -> JsonResult<Vec<EnabledFeature>> {
    let AuthGuard { user, .. } = auth;

    let mut features = vec![];

    features.extend(ticket::get_enabled_features_by_user(&mut conn, &user).await);
    features.extend(role::get_enabled_features_by_user(&mut conn, &user).await);

    Ok(Json(features))
}

pub fn routes() -> Vec<Route> {
    routes![
        ping,
        all_projects,
        get_project_by_id,
        get_auth_project,
        login,
        verify_token,
        get_me,
        logout,
        get_features_by_user
    ]
}
