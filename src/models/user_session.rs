use chrono::NaiveDateTime;
use diesel::dsl::now;
use diesel::prelude::*;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use uuid::Uuid;

use crate::{
    modules::guard::AuthGuard,
    schema::{projects, user_sessions, users},
};

use super::{project::Project, user::User};

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, AsChangeset, Clone)]
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
        let id = Uuid::new_v4().to_string();

        let _ = diesel::insert_into(user_sessions::table)
            .values((
                user_sessions::id.eq(id.clone()),
                user_sessions::user_id.eq(user_id),
                user_sessions::user_agent.eq(user_agent),
                user_sessions::ip.eq(ip),
                user_sessions::expired_at
                    .eq(chrono::Utc::now().naive_utc() + chrono::Duration::days(7)),
            ))
            .execute(conn)
            .await;

        user_sessions::table
            .find(id.clone())
            .select(UserSession::as_select())
            .first(conn)
            .await
    }

    pub async fn auth(
        conn: &mut crate::DbConn,
        id: String,
    ) -> Result<AuthGuard, diesel::result::Error> {
        let (project, user, user_session) = user_sessions::table
            .find(id)
            .filter(now.lt(user_sessions::expired_at))
            .inner_join(users::table.inner_join(projects::table))
            .select((
                Project::as_select(),
                User::as_select(),
                UserSession::as_select(),
            ))
            .first(conn)
            .await?;
        Ok(AuthGuard {
            project,
            user,
            user_session,
        })
    }

    pub async fn expire(&self, conn: &mut crate::DbConn) -> Result<usize, diesel::result::Error> {
        diesel::update(user_sessions::table.filter(user_sessions::id.eq(self.id.clone())))
            .set(user_sessions::expired_at.eq(now))
            .execute(conn)
            .await
    }
}
