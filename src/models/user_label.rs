use chrono::NaiveDateTime;

use crate::models::label::Label;
use crate::models::user::User;
use crate::schema::users_labels;
use crate::utils::serde::unix_time;

#[derive(
    Queryable,
    Identifiable,
    Selectable,
    Associations,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Label))]
#[diesel(table_name = users_labels)]
#[diesel(primary_key(user_id, label_id))]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct UserLabel {
    pub user_id: String,
    pub label_id: i32,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}
