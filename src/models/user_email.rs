use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::models::user::User;
use crate::schema::{user_emails, users};
use crate::utils::serde::unix_time;

#[derive(
    Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(User))]
#[diesel(table_name = user_emails)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct UserEmail {
    pub id: i32,
    pub user_id: String,
    pub email: String,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl UserEmail {
    pub async fn get_user(
        conn: &mut crate::DbConn,
        project_id: String,
        email: String,
    ) -> Result<User, diesel::result::Error> {
        user_emails::table
            .filter(user_emails::email.eq(email))
            .inner_join(users::table)
            .filter(users::project_id.eq(project_id))
            .select(User::as_select())
            .first(conn)
            .await
    }
}
