use chrono::NaiveDateTime;
use diesel::dsl::exists;
use diesel::{prelude::*, select};
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::models::{project::Project, user::User};
use crate::schema::{labels, users, users_labels};
use crate::utils::serde::unix_time;

use super::user_label::UserLabel;

#[derive(
    Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Deserialize,
)]
#[diesel(table_name = labels)]
#[diesel(belongs_to(Project))]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Label {
    pub id: i32,
    pub project_id: String,
    pub key: String,
    pub value: String,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl Label {
    pub async fn find_or_create(
        conn: &mut crate::DbConn,
        project_id: String,
        key: String,
        value: String,
    ) -> Result<Label, diesel::result::Error> {
        let label = labels::table
            .filter(labels::project_id.eq(project_id.clone()))
            .filter(labels::key.eq(key.clone()))
            .filter(labels::value.eq(value.clone()))
            .first(conn)
            .await;

        match label {
            Ok(label) => Ok(label),
            Err(_) => {
                sql_function! {
                    fn last_insert_id() -> Integer;
                }

                diesel::insert_into(labels::table)
                    .values((
                        labels::project_id.eq(project_id),
                        labels::key.eq(key),
                        labels::value.eq(value),
                    ))
                    .execute(conn)
                    .await?;

                labels::table.find(last_insert_id()).first(conn).await
            }
        }
    }

    pub async fn get_users(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<User>, diesel::result::Error> {
        UserLabel::belonging_to(self)
            .inner_join(users::table)
            .select(User::as_select())
            .load(conn)
            .await
    }

    pub async fn is_user(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<bool, diesel::result::Error> {
        select(exists(
            users_labels::table
                .filter(users_labels::user_id.eq(user.id.clone()))
                .filter(users_labels::label_id.eq(self.id)),
        ))
        .get_result(conn)
        .await
    }
}
