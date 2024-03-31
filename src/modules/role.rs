use crate::models::label::Label;
use crate::models::role::Role;
use crate::modules::JsonResult;
use crate::DbConn;
use crate::{error::AppError, models::user::User};
use rocket::serde::json::{json, Json, Value};
use rocket::Route;
use rocket_db_pools::diesel::AsyncConnection;

use super::{guard::AuthGuard, ApiResult, EmptyResponse, EmptyResult, EnabledFeature};

#[get("/role/<role_id>")]
async fn get_role(mut conn: DbConn, role_id: String) -> ApiResult<Value> {
    let role = Role::find(&mut conn, role_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    Ok(json!({
        "name_zh": role.name_zh,
        "name_en": role.name_en,
        "login_message_zh": role.login_message_zh,
        "login_message_en": role.login_message_en,
    }))
}

#[get("/role/roles")]
async fn all_roles(mut conn: DbConn, auth: AuthGuard) -> JsonResult<Vec<Role>> {
    let AuthGuard { user, .. } = auth;
    Ok(Role::get_roles_by_user(&mut conn, &user)
        .await
        .map_or(Json(vec![]), Json))
}

#[get("/role/admin/roles")]
async fn all_roles_in_admin(mut conn: DbConn, auth: AuthGuard) -> JsonResult<Vec<Role>> {
    let AuthGuard { user, .. } = auth;
    Ok(Role::get_manage_roles_by_user(&mut conn, &user)
        .await
        .map_or(Json(vec![]), Json))
}

#[derive(Deserialize)]
struct AdminRoleReq {
    pub name_zh: Option<String>,
    pub name_en: Option<String>,
    pub login_message_zh: Option<String>,
    pub login_message_en: Option<String>,
    pub welcome_message_zh: Option<String>,
    pub welcome_message_en: Option<String>,
}

#[put("/role/admin/roles/<role_id>", data = "<role_req>")]
async fn put_role_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    role_id: String,
    role_req: Json<AdminRoleReq>,
) -> EmptyResult {
    let AuthGuard { user, .. } = auth;
    let mut role = Role::find(&mut conn, role_id.clone())
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    match role.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not a manager of this role".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    if let Some(name_zh) = role_req.name_zh.clone() {
        role.name_zh = name_zh;
    }
    if let Some(login_message_zh) = role_req.login_message_zh.clone() {
        role.login_message_zh = Some(login_message_zh);
    }
    if let Some(welcome_message_zh) = role_req.welcome_message_zh.clone() {
        role.welcome_message_zh = Some(welcome_message_zh);
    }

    if let Some(name_en) = role_req.name_en.clone() {
        role.name_en = name_en;
    }
    if let Some(login_message_en) = role_req.login_message_en.clone() {
        role.login_message_en = Some(login_message_en);
    }
    if let Some(welcome_message_en) = role_req.welcome_message_en.clone() {
        role.welcome_message_en = Some(welcome_message_en);
    }

    role.save(&mut conn)
        .await
        .map(|_| EmptyResponse)
        .map_err(|err| AppError::internal(err.to_string()))
}

#[get("/role/admin/roles/<role_id>/users")]
async fn all_role_users_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    role_id: String,
) -> JsonResult<Vec<User>> {
    let AuthGuard { user, .. } = auth;
    let role = Role::find(&mut conn, role_id.clone())
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    match role.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not a manager of this role".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    Ok(role.get_users(&mut conn).await.map_or(Json(vec![]), Json))
}

#[derive(Deserialize)]
struct AdminAddRoleUser {
    pub name: String,
    pub locale: String,
    pub emails: Vec<String>,
}

#[post("/role/admin/roles/<role_id>/users", data = "<add_role_user_req>")]
async fn add_role_users_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    role_id: String,
    add_role_user_req: Json<Vec<AdminAddRoleUser>>,
) -> EmptyResult {
    let AuthGuard { user, project, .. } = auth;
    let role = Role::find(&mut conn, role_id.clone())
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    match role.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not a manager of this role".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    conn.transaction(|mut conn| {
        Box::pin(async move {
            for user_req in add_role_user_req.iter() {
                let user = project
                    .add_user(&mut conn, user_req.name.clone(), user_req.locale.clone())
                    .await?;

                let _ = user.add_emails(&mut conn, &user_req.emails).await?;

                let label = Label::find_or_create(
                    &mut conn,
                    project.id.clone(),
                    "role".to_owned(),
                    role.id.clone(),
                )
                .await?;

                let _ = user.add_label(&mut conn, &label).await?;
            }

            Ok::<_, diesel::result::Error>(EmptyResponse)
        })
    })
    .await
    .map(|_| EmptyResponse)
    .map_err(|err| AppError::internal(err.to_string()))
}

#[delete("/role/admin/roles/<role_id>/users/<user_id>")]
async fn delete_role_user_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    role_id: String,
    user_id: String,
) -> EmptyResult {
    let AuthGuard { user, .. } = auth;
    let role = Role::find(&mut conn, role_id.clone())
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    match role.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not a manager of this role".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    let delete_user = User::find(&mut conn, user_id.clone())
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    conn.transaction(|mut conn| {
        Box::pin(async move {
            let label = role.get_label(&mut conn).await?;

            let _ = delete_user.delete_label(&mut conn, &label).await?;

            Ok::<_, diesel::result::Error>(EmptyResponse)
        })
    })
    .await
    .map(|_| EmptyResponse)
    .map_err(|err| AppError::internal(err.to_string()))
}

pub async fn get_enabled_features_by_user(conn: &mut DbConn, user: &User) -> Vec<EnabledFeature> {
    let manager_roles = Role::get_manage_roles_by_user(conn, &user)
        .await
        .unwrap_or(vec![]);

    if manager_roles.is_empty() {
        vec![]
    } else {
        vec![EnabledFeature::RoleManage(manager_roles.len())]
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        get_role,
        all_roles,
        all_roles_in_admin,
        put_role_in_admin,
        all_role_users_in_admin,
        add_role_users_in_admin,
        delete_role_user_in_admin
    ]
}
