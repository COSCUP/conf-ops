use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::models::user::User;
use crate::schema::{user_emails, users};
use crate::utils::serde::unixtime;

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
    #[serde(with = "unixtime")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unixtime")]
    pub updated_at: NaiveDateTime,
}

impl UserEmail {
    pub async fn get_user(
        conn: &mut crate::DbConn,
        email: String,
    ) -> Result<User, diesel::result::Error> {
        user_emails::table
            .filter(user_emails::email.eq(email))
            .inner_join(users::table)
            .select(User::as_select())
            .first(conn)
            .await
    }
}