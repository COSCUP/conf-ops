use chrono::NaiveDateTime;
use diesel::{
    dsl::{Eq, Filter, InnerJoin, IntoBoxed, Select},
    mysql::Mysql,
    prelude::*,
};
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use uuid::Uuid;

use crate::{
    models::project::Project,
    schema::{labels, user_emails, users, users_labels},
    utils::serde::unix_time,
    DbConn,
};

use super::{label::Label, user_label::UserLabel};

#[derive(
    Queryable,
    Identifiable,
    Selectable,
    Associations,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Eq,
    Hash,
    AsChangeset,
    Clone,
    Insertable
)]
#[diesel(belongs_to(Project))]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct User {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub locale: String,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl User {
    pub async fn create(
        conn: &mut DbConn,
        name: String,
        project_id: String,
        locale: String,
    ) -> Result<User, diesel::result::Error> {
        let id = Uuid::new_v4().to_string().to_uppercase();

        let _ = diesel::insert_into(users::table)
            .values((
                users::id.eq(id.clone()),
                users::name.eq(name),
                users::project_id.eq(project_id),
                users::locale.eq(locale),
            ))
            .execute(conn)
            .await;

        users::table
            .find(id)
            .select(User::as_select())
            .first(conn)
            .await
    }

    pub async fn find(conn: &mut crate::DbConn, id: String) -> Result<User, diesel::result::Error> {
        users::table.find(id).first(conn).await
    }

    pub async fn get_emails(
        &self,
        conn: &mut DbConn,
    ) -> Result<Vec<String>, diesel::result::Error> {
        user_emails::table
            .filter(user_emails::user_id.eq(self.id.to_owned()))
            .select(user_emails::email)
            .load(conn)
            .await
    }

    pub async fn add_emails(
        &self,
        conn: &mut DbConn,
        email: &Vec<String>,
    ) -> Result<usize, diesel::result::Error> {
        diesel::insert_into(user_emails::table)
            .values(
                email
                    .iter()
                    .map(|email| {
                        (
                            user_emails::user_id.eq(self.id.to_owned()),
                            user_emails::email.eq(email),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(conn)
            .await
    }

    pub async fn remove_emails(
        &self,
        conn: &mut DbConn
    ) -> Result<usize, diesel::result::Error> {
        diesel::delete(
            user_emails::table
                .filter(user_emails::user_id.eq(self.id.to_owned()))
        )
        .execute(conn)
        .await
    }

    pub async fn get_labels(&self, conn: &mut DbConn) -> Result<Vec<Label>, diesel::result::Error> {
        UserLabel::belonging_to(self)
            .inner_join(labels::table)
            .select(Label::as_select())
            .load(conn)
            .await
    }

    pub fn build_role_ids_query<'a>(
        &self,
    ) -> IntoBoxed<
        'a,
        Select<
            Filter<
                InnerJoin<
                    Filter<users_labels::table, Eq<users_labels::user_id, String>>,
                    labels::table,
                >,
                Eq<labels::key, String>,
            >,
            labels::value,
        >,
        Mysql,
    > {
        users_labels::table
            .filter(users_labels::user_id.eq(self.id.clone()))
            .inner_join(labels::table)
            .filter(labels::key.eq("role"))
            .select(labels::value)
            .into_boxed()
    }

    pub fn build_user_labels_query<'a>(
        &self,
    ) -> IntoBoxed<
        'a,
        Select<
            InnerJoin<
                Filter<users_labels::table, Eq<users_labels::user_id, String>>,
                labels::table,
            >,
            labels::id,
        >,
        Mysql,
    > {
        users_labels::table
            .filter(users_labels::user_id.eq(self.id.clone()))
            .inner_join(labels::table)
            .select(labels::id)
            .into_boxed()
    }

    pub async fn add_label(
        &self,
        conn: &mut DbConn,
        label: &Label,
    ) -> Result<usize, diesel::result::Error> {
        diesel::insert_into(users_labels::table)
            .values((
                users_labels::user_id.eq(self.id.to_owned()),
                users_labels::label_id.eq(label.id),
            ))
            .execute(conn)
            .await
    }

    pub async fn delete_label(
        &self,
        conn: &mut DbConn,
        label: &Label,
    ) -> Result<usize, diesel::result::Error> {
        diesel::delete(
            users_labels::table
                .filter(users_labels::user_id.eq(self.id.to_owned()))
                .filter(users_labels::label_id.eq(label.id)),
        )
        .execute(conn)
        .await
    }

    pub async fn save(&self, conn: &mut DbConn) -> Result<usize, diesel::result::Error> {
        match diesel::replace_into(users::table)
            .values(self)
            .execute(conn)
            .await
        {
            Ok(result) => Ok(result),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                _,
            )) => {
                diesel::update(users::table)
                    .filter(users::id.eq(&self.id))
                    .set(self)
                    .execute(conn)
                    .await
            }
            Err(e) => Err(e),
        }
    }
}
