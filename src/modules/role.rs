use crate::models::label::Label;
use crate::models::project::Project;
use crate::models::role::Role;
use crate::models::user::AuthUser;
use crate::modules::JsonResult;
use crate::DbConn;
use crate::{error::AppError, models::user::User};
use rocket::serde::json::Json;
use rocket::Route;
use rocket_db_pools::diesel::AsyncConnection;

use super::{EmptyResponse, EmptyResult};

#[get("/project/role/roles")]
pub async fn all_roles(mut conn: DbConn, user: AuthUser) -> JsonResult<Vec<Role>> {
    Ok(Role::get_manage_roles_by_user(&mut conn, user)
        .await
        .map_or(Json(vec![]), Json))
}

#[derive(Deserialize)]
pub struct RoleReq {
    pub name: Option<String>,
    pub login_message: Option<String>,
    pub welcome_message: Option<String>,
}

#[put("/project/role/roles/<role_id>", data = "<role_req>")]
pub async fn put_role(
    mut conn: DbConn,
    _user: AuthUser,
    role_id: String,
    role_req: Json<RoleReq>,
) -> EmptyResult {
    let mut role = Role::find(&mut conn, role_id.clone())
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    if let Some(name) = role_req.name.clone() {
        role.name = name;
    }
    if let Some(login_message) = role_req.login_message.clone() {
        role.login_message = Some(login_message);
    }
    if let Some(welcome_message) = role_req.welcome_message.clone() {
        role.welcome_message = Some(welcome_message);
    }

    role.save(&mut conn)
        .await
        .map(|_| EmptyResponse)
        .map_err(|err| AppError::internal(err.to_string()))
}

#[get("/project/role/roles/<role_id>/users")]
pub async fn all_role_users(
    mut conn: DbConn,
    _user: AuthUser,
    role_id: String,
) -> JsonResult<Vec<AuthUser>> {
    let role = Role::find(&mut conn, role_id.clone())
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    Ok(role.get_users(&mut conn).await.map_or(Json(vec![]), Json))
}

#[derive(Deserialize)]
pub struct AddRoleUser {
    pub name: String,
    pub emails: Vec<String>,
}

#[post("/project/role/roles/<role_id>/users", data = "<add_role_user_req>")]
pub async fn add_role_users(
    mut conn: DbConn,
    project: Project,
    _user: AuthUser,
    role_id: String,
    add_role_user_req: Json<Vec<AddRoleUser>>,
) -> EmptyResult {
    let role = Role::find(&mut conn, role_id.clone())
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    conn.transaction(|mut conn| {
        Box::pin(async move {
            for user_req in add_role_user_req.iter() {
                let user = project.add_user(&mut conn, user_req.name.clone()).await?;

                let _ = user.add_emails(&mut conn, user_req.emails.clone()).await?;

                let label = Label::find_or_create(
                    &mut conn,
                    project.id.clone(),
                    "role".to_owned(),
                    role.id.clone(),
                )
                .await?;

                let _ = user.add_label(&mut conn, label).await?;
            }

            Ok::<_, diesel::result::Error>(EmptyResponse)
        })
    })
    .await
    .map(|_| EmptyResponse)
    .map_err(|err| AppError::internal(err.to_string()))
}

#[delete("/project/role/roles/<role_id>/users/<user_id>")]
pub async fn delete_role_user(
    mut conn: DbConn,
    _user: AuthUser,
    role_id: String,
    user_id: String,
) -> EmptyResult {
    let role = Role::find(&mut conn, role_id.clone())
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    let user = User::find(&mut conn, user_id.clone())
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    conn.transaction(|mut conn| {
        Box::pin(async move {
            let label = role.get_label(&mut conn).await?;

            let _ = user.delete_label(&mut conn, label).await?;

            Ok::<_, diesel::result::Error>(EmptyResponse)
        })
    })
    .await
    .map(|_| EmptyResponse)
    .map_err(|err| AppError::internal(err.to_string()))
}

pub fn routes() -> Vec<Route> {
    routes![
        all_roles,
        put_role,
        all_role_users,
        add_role_users,
        delete_role_user
    ]
}
