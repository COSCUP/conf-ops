use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::{
    models::{label::Label, project::Project, target::Target, user::User},
    schema::{labels, role_managers, roles, targets},
    utils::serde::unix_time,
};

#[derive(
    Queryable,
    Identifiable,
    Selectable,
    Associations,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Insertable,
)]
#[diesel(belongs_to(Project))]
#[diesel(table_name = roles)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Role {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub login_message: Option<String>,
    pub welcome_message: Option<String>,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

#[derive(
    Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(Role))]
#[diesel(belongs_to(Target))]
#[diesel(table_name = role_managers)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct RoleManager {
    pub id: i32,
    pub role_id: String,
    pub target_id: i32,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl Role {
    pub async fn find(conn: &mut crate::DbConn, id: String) -> Result<Role, diesel::result::Error> {
        roles::table.find(id).first(conn).await
    }

    pub async fn save(&self, conn: &mut crate::DbConn) -> Result<usize, diesel::result::Error> {
        diesel::replace_into(roles::table)
            .values(self)
            .execute(conn)
            .await
    }

    pub async fn get_roles_by_user(
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<Vec<Role>, diesel::result::Error> {
        let role_ids = user.build_role_ids_query("role".to_owned());

        roles::table
            .filter(roles::id.eq_any(role_ids))
            .select(Role::as_select())
            .load(conn)
            .await
    }

    pub async fn get_manage_roles_by_user(
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<Vec<Role>, diesel::result::Error> {
        let user_label_ids = user.build_user_labels_query("role".to_owned());

        role_managers::table
            .inner_join(roles::table)
            .inner_join(targets::table.left_join(labels::table))
            .filter(labels::id.eq_any(user_label_ids))
            .or_filter(targets::user_id.eq(user.id.clone()))
            .select(Role::as_select())
            .load(conn)
            .await
    }

    pub async fn get_label(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Label, diesel::result::Error> {
        Label::find_or_create(
            conn,
            self.project_id.clone(),
            "role".to_owned(),
            self.id.clone(),
        )
        .await
    }

    pub async fn get_users(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<User>, diesel::result::Error> {
        let label = self.get_label(conn).await?;

        label.get_users(conn).await
    }

    pub async fn is_user(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<bool, diesel::result::Error> {
        let label: Label = self.get_label(conn).await?;

        label.is_user(conn, user).await
    }

    pub async fn get_managers(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<User>, diesel::result::Error> {
        let managers: Vec<Target> = RoleManager::belonging_to(self)
            .inner_join(targets::table)
            .select(Target::as_select())
            .load(conn)
            .await?;

        Target::get_users(conn, &managers).await
    }

    pub async fn is_manager(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<bool, diesel::result::Error> {
        let managers: Vec<Target> = RoleManager::belonging_to(self)
            .inner_join(targets::table)
            .select(Target::as_select())
            .load(conn)
            .await?;

        Target::is_user_in_targets(conn, user, &managers).await
    }
}
