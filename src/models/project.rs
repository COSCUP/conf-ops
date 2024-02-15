use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket::request::FromRequest;
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::{
    error::AppError, models::{label::Label, user::User}, schema::{labels::{self}, projects, users}, utils::serde::unixtime, DbConn
};

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = projects)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(with = "unixtime")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unixtime")]
    pub updated_at: NaiveDateTime,
}

impl Project {
    pub async fn all(conn: &mut crate::DbConn) -> Result<Vec<Project>, diesel::result::Error> {
        projects::table
            .select(Project::as_select())
            .load(conn)
            .await
    }

    pub async fn get(
        conn: &mut crate::DbConn,
        id: String,
    ) -> Result<Project, diesel::result::Error> {
        projects::table
            .filter(projects::id.eq(id))
            .select(Project::as_select())
            .first(conn)
            .await
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
    ) -> Result<usize, diesel::result::Error> {
        diesel::insert_into(users::table)
            .values((
                users::name.eq(name),
                users::project_id.eq(self.id.to_owned()),
            ))
            .execute(conn)
            .await
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

    pub async fn add_label(
        &self,
        conn: &mut crate::DbConn,
        key: String,
        value: String,
    ) -> Result<usize, diesel::result::Error> {
        diesel::insert_into(labels::table)
            .values((
                labels::project_id.eq(self.id.to_owned()),
                labels::key.eq(key),
                labels::value.eq(value),
            ))
            .execute(conn)
            .await
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Project {
    type Error = AppError;

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let mut db = match request.guard::<DbConn>().await {
            rocket::outcome::Outcome::Success(db) => db,
            rocket::outcome::Outcome::Error(error) => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::InternalServerError,
                    AppError::internal(error.1.unwrap().to_string()),
                ))
            }
            rocket::outcome::Outcome::Forward(s) => return rocket::outcome::Outcome::Forward(s),
        };

        let project_not_found_error = rocket::request::Outcome::Error((
            rocket::http::Status::NotFound,
            AppError::not_found("Project ID not found".to_owned()),
        ));

        let project_id: String = match request.headers().get_one("x-project-id") {
            Some(text) => text.to_owned(),
            _ => {
                return project_not_found_error
            }
        };

        match Project::get(&mut db, project_id).await {
            Ok(project) => rocket::request::Outcome::Success(project),
            Err(_) => project_not_found_error
        }
    }
}
