use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::{
    error::AppError,
    models::{label::Label, user::User},
    schema::{
        labels::{self},
        projects,
    },
    utils::serde::unix_time,
    DbConn,
};

#[derive(
    Queryable,
    Identifiable,
    Selectable,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    AsChangeset,
    Clone,
)]
#[diesel(table_name = projects)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Project {
    pub id: String,
    pub name_zh: String,
    pub description_zh: String,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
    pub name_en: String,
    pub description_en: String,
}

impl Project {
    pub async fn all(conn: &mut crate::DbConn) -> Result<Vec<Project>, diesel::result::Error> {
        projects::table
            .order(projects::created_at.asc())
            .select(Project::as_select())
            .load(conn)
            .await
    }

    pub async fn find(
        conn: &mut crate::DbConn,
        id: String,
    ) -> Result<Project, diesel::result::Error> {
        projects::table.find(id).first(conn).await
    }

    pub async fn get_users(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<User>, diesel::result::Error> {
        User::belonging_to(self)
            .select(User::as_select())
            .load(conn)
            .await
    }

    pub async fn add_user(
        &self,
        conn: &mut crate::DbConn,
        name: String,
        locale: String,
    ) -> Result<User, diesel::result::Error> {
        User::create(conn, name, self.id.clone(), locale).await
    }

    pub async fn get_labels(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<Label>, diesel::result::Error> {
        Label::belonging_to(self)
            .select(Label::as_select())
            .load(conn)
            .await
    }

    pub async fn get_labels_by_key(
        &self,
        conn: &mut crate::DbConn,
        key: String,
    ) -> Result<Vec<Label>, diesel::result::Error> {
        Label::belonging_to(self)
            .filter(labels::key.eq(key))
            .select(Label::as_select())
            .load(conn)
            .await
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Project {
    type Error = AppError;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let project_result = request
            .local_cache_async(async move {
                let mut db = try_outcome!(request.guard::<DbConn>().await.map_error(|(s, e)| (
                    s,
                    AppError::internal(
                        e.map_or("Unknown database problem".to_owned(), |err| err.to_string())
                    )
                )));

                let project_not_found_error = Outcome::Error((
                    rocket::http::Status::NotFound,
                    AppError::not_found("Project ID not found".to_owned()),
                ));

                let project_id: String = match request.headers().get_one("x-project-id") {
                    Some(text) => text.to_owned(),
                    _ => return project_not_found_error,
                };

                match Project::find(&mut db, project_id).await {
                    Ok(project) => Outcome::Success(project),
                    Err(_) => project_not_found_error,
                }
            })
            .await;

        match project_result {
            Outcome::Success(project) => Outcome::Success(project.clone()),
            Outcome::Error((s, e)) => Outcome::Error((*s, e.clone())),
            Outcome::Forward(f) => Outcome::Forward(*f),
        }
    }
}
