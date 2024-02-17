use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket::request::FromRequest;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{label::Label, user_label::UserLabel};

use crate::{
    models::project::Project,
    schema::{labels, user_emails, users, users_labels},
    utils::serde::unix_time,
    DbConn,
};

use super::user_session::UserSession;

#[derive(
    Queryable,
    Identifiable,
    Selectable,
    Associations,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Eq,
    Hash,
)]
#[diesel(belongs_to(Project))]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct User {
    pub id: String,
    pub name: String,
    pub project_id: String,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl User {
    pub async fn create(
        conn: &mut DbConn,
        name: String,
        project_id: String,
    ) -> Result<User, diesel::result::Error> {
        let id = Uuid::new_v4().to_string();

        let _ = diesel::insert_into(users::table)
            .values((
                users::id.eq(id.clone()),
                users::name.eq(name),
                users::project_id.eq(project_id),
            ))
            .execute(conn)
            .await;

        users::table
            .find(id)
            .select(User::as_select())
            .first(conn)
            .await
    }

    pub async fn find(conn: &mut crate::DbConn, id: String) -> Result<User, diesel::result::Error> {
        users::table.find(id).first(conn).await
    }

    pub async fn get_emails(
        &self,
        conn: &mut DbConn,
    ) -> Result<Vec<String>, diesel::result::Error> {
        user_emails::table
            .filter(user_emails::user_id.eq(self.id.to_owned()))
            .select(user_emails::email)
            .load(conn)
            .await
    }

    pub async fn add_emails(
        &self,
        conn: &mut DbConn,
        email: Vec<String>,
    ) -> Result<usize, diesel::result::Error> {
        diesel::insert_into(user_emails::table)
            .values(
                email
                    .iter()
                    .map(|email| {
                        (
                            user_emails::user_id.eq(self.id.to_owned()),
                            user_emails::email.eq(email),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(conn)
            .await
    }

    pub async fn get_labels(&self, conn: &mut DbConn) -> Result<Vec<Label>, diesel::result::Error> {
        UserLabel::belonging_to(self)
            .inner_join(labels::table)
            .select(Label::as_select())
            .load(conn)
            .await
    }

    pub async fn get_labels_by_key(
        &self,
        conn: &mut DbConn,
        key: String,
    ) -> Result<Vec<Label>, diesel::result::Error> {
        UserLabel::belonging_to(self)
            .inner_join(labels::table)
            .filter(labels::key.eq(key))
            .select(Label::as_select())
            .load(conn)
            .await
    }

    pub async fn add_label(
        &self,
        conn: &mut DbConn,
        label: Label,
    ) -> Result<usize, diesel::result::Error> {
        diesel::insert_into(users_labels::table)
            .values((
                users_labels::user_id.eq(self.id.to_owned()),
                users_labels::label_id.eq(label.id),
            ))
            .execute(conn)
            .await
    }

    pub async fn delete_label(
        &self,
        conn: &mut DbConn,
        label: Label,
    ) -> Result<usize, diesel::result::Error> {
        diesel::delete(
            users_labels::table
                .filter(users_labels::user_id.eq(self.id.to_owned()))
                .filter(users_labels::label_id.eq(label.id)),
        )
        .execute(conn)
        .await
    }

}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = AppError;

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let mut db = match request.guard::<DbConn>().await {
            rocket::outcome::Outcome::Success(db) => db,
            rocket::outcome::Outcome::Error(error) => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::InternalServerError,
                    AppError::internal(
                        error
                            .1
                            .map_or("Unknown database problem".to_owned(), |err| err.to_string()),
                    ),
                ))
            }
            rocket::outcome::Outcome::Forward(s) => return rocket::outcome::Outcome::Forward(s),
        };

        let cookie = match request.cookies().get_private("session_id") {
            Some(cookie) => cookie,
            None => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::Unauthorized,
                    AppError::unauthorized(),
                ))
            }
        };

        let session_id = match cookie.value().parse() {
            Ok(session_id) => session_id,
            _ => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::Unauthorized,
                    AppError::unauthorized(),
                ))
            }
        };

        let user = match UserSession::auth_user(&mut db, session_id).await {
            Ok(user) => user,
            Err(_) => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::Unauthorized,
                    AppError::unauthorized(),
                ))
            }
        };

        let project = match request.guard::<Project>().await {
            rocket::outcome::Outcome::Success(project) => project,
            rocket::outcome::Outcome::Error(error) => {
                return rocket::request::Outcome::Error(error)
            }
            rocket::outcome::Outcome::Forward(s) => return rocket::outcome::Outcome::Forward(s),
        };

        if user.project_id != project.id {
            return rocket::request::Outcome::Error((
                rocket::http::Status::Unauthorized,
                AppError::unauthorized(),
            ));
        }

        rocket::outcome::Outcome::Success(user)
    }
}

pub type AuthUser = User;
