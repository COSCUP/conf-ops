use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::models::project::Project;
use crate::schema::labels;
use crate::utils::serde::unixtime;

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
    #[serde(with = "unixtime")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unixtime")]
    pub updated_at: NaiveDateTime,
}
