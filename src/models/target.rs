use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::schema::{labels, users};
use crate::utils::serde::unix_time;
use crate::utils::vec::UniqueVec;
use crate::{
    models::{label::Label, user::User},
    schema::targets,
};

use super::user_label::UserLabel;

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

impl Target {
    pub async fn find(conn: &mut crate::DbConn, id: i32) -> Result<Target, diesel::result::Error> {
        targets::table.find(id).first(conn).await
    }

    pub async fn find_or_create_user(
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<Target, diesel::result::Error> {
        let target = targets::table
            .filter(targets::user_id.eq(Some(user.id.clone())))
            .first::<Target>(conn)
            .await;

        match target {
            Ok(target) => Ok(target),
            Err(_) => {
                let _ = diesel::insert_into(targets::table)
                    .values(targets::user_id.eq(user.id.clone()))
                    .execute(conn)
                    .await?;

                sql_function! {
                    fn last_insert_id() -> Integer;
                }

                targets::table.find(last_insert_id()).first(conn).await
            }
        }
    }

    pub async fn get_users(
        conn: &mut crate::DbConn,
        targets: &Vec<Target>,
    ) -> Result<Vec<User>, diesel::result::Error> {
        let list: Vec<(Option<User>, Option<Label>)> = targets::table
            .filter(targets::id.eq_any(targets.iter().map(|t| t.id).collect::<Vec<i32>>()))
            .left_join(users::table)
            .left_join(labels::table)
            .select((Option::<User>::as_select(), Option::<Label>::as_select()))
            .load::<(Option<User>, Option<Label>)>(conn)
            .await?;

        let labels = list
            .iter()
            .filter_map(|(_, labels)| labels.as_ref().clone())
            .collect::<Vec<&Label>>();

        let role_users: Vec<_> = UserLabel::belonging_to(&labels)
            .inner_join(users::table)
            .select(User::as_select())
            .load(conn)
            .await?;

        let direct_users = list
            .into_iter()
            .filter_map(|(user, _)| user)
            .collect::<Vec<User>>();

        let mut users: Vec<_> = role_users
            .into_iter()
            .chain(direct_users.into_iter())
            .collect();
        users.unique_by_key(|u| u.id.clone());

        Ok(users)
    }

    pub async fn is_user_in_targets(
        conn: &mut crate::DbConn,
        user: &User,
        list: &Vec<Target>,
    ) -> Result<bool, diesel::result::Error> {
        let user_label_ids = user.build_user_labels_query().load(conn).await?;

        Ok(list.iter().any(|t| {
            if let Some(label_id) = t.label_id {
                if user_label_ids.contains(&label_id) {
                    return true;
                }
            }

            if let Some(user_id) = &t.user_id {
                if user_id == &user.id {
                    return true;
                }
            }
            false
        }))
    }
}
