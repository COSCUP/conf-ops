use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::utils::serde::unix_time;
use crate::{
    models::{label::Label, user::User},
    schema::targets,
};

#[derive(
    Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Label))]
#[diesel(table_name = targets)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Target {
    pub id: i32,
    pub user_id: Option<String>,
    pub label_id: Option<i32>,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}
