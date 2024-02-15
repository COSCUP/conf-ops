use chrono::NaiveDateTime;
use diesel::dsl::now;
use diesel::prelude::*;
use rocket::request::FromRequest;
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::error::AppError;
use crate::models::user::User;
use crate::schema::{user_sessions, users};
use crate::DbConn;

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(User))]
#[diesel(table_name = user_sessions)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct UserSession {
    pub id: String,
    pub user_id: String,
    pub user_agent: String,
    pub ip: String,
    pub expired_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl UserSession {
    pub async fn create(
        conn: &mut crate::DbConn,
        user_id: String,
        user_agent: String,
        ip: String,
    ) -> Result<UserSession, diesel::result::Error> {
        let _ = diesel::insert_into(user_sessions::table)
            .values((
                user_sessions::user_id.eq(user_id),
                user_sessions::user_agent.eq(user_agent),
                user_sessions::ip.eq(ip),
                user_sessions::expired_at
                    .eq(chrono::Utc::now().naive_utc() + chrono::Duration::days(7)),
            ))
            .execute(conn)
            .await;

        user_sessions::table
            .order(user_sessions::id.desc())
            .select(UserSession::as_select())
            .first(conn)
            .await
    }

    pub async fn auth_user(
        conn: &mut crate::DbConn,
        id: String,
    ) -> Result<User, diesel::result::Error> {
        user_sessions::table
            .filter(user_sessions::id.eq(id))
            .filter(now.lt(user_sessions::expired_at))
            .inner_join(users::table)
            .select(User::as_select())
            .first(conn)
            .await
    }

    pub async fn auth(
        conn: &mut crate::DbConn,
        id: String,
    ) -> Result<UserSession, diesel::result::Error> {
        user_sessions::table
            .filter(user_sessions::id.eq(id))
            .filter(now.lt(user_sessions::expired_at))
            .select(UserSession::as_select())
            .first(conn)
            .await
    }

    pub async fn expire (
        &self,
        conn: &mut crate::DbConn
    ) -> Result<usize, diesel::result::Error> {
        diesel::update(user_sessions::table.filter(user_sessions::id.eq(self.id.clone())))
            .set(user_sessions::expired_at.eq(now))
            .execute(conn)
            .await
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserSession {
    type Error = AppError;

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let mut db = match request.guard::<DbConn>().await {
            rocket::outcome::Outcome::Success(db) => db,
            rocket::outcome::Outcome::Error(error) => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::InternalServerError,
                    AppError::internal(error.1.map_or("Unknown database problem".to_owned(),|err| err.to_string())),
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

        match UserSession::auth(&mut db, session_id).await {
            Ok(user_session) => rocket::outcome::Outcome::Success(user_session),
            Err(_) => {
                return rocket::request::Outcome::Error((
                    rocket::http::Status::Unauthorized,
                    AppError::unauthorized(),
                ))
            }
        }
    }
}

