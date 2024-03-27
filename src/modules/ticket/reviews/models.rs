use chrono::NaiveDateTime;
use diesel::prelude::*;
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::modules::ticket::models::{TicketFlow, TicketSchemaFlow};
use crate::schema::{ticket_reviews, ticket_schema_reviews};
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
    Insertable,
    AsChangeset,
)]
#[diesel(belongs_to(TicketSchemaFlow))]
#[diesel(table_name = ticket_schema_reviews)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketSchemaReview {
    pub id: i32,
    pub ticket_schema_flow_id: i32,
    pub restarted: bool,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketSchemaReview {
    pub async fn create(
        conn: &mut crate::DbConn,
        schema_flow: &TicketSchemaFlow,
        restarted: bool,
    ) -> Result<TicketSchemaReview, diesel::result::Error> {
        diesel::insert_into(ticket_schema_reviews::table)
            .values((
                ticket_schema_reviews::ticket_schema_flow_id.eq(schema_flow.id),
                ticket_schema_reviews::restarted.eq(restarted),
            ))
            .execute(conn)
            .await?;

        sql_function! {
            fn last_insert_id() -> Integer;
        }

        ticket_schema_reviews::table
            .find(last_insert_id())
            .first(conn)
            .await
    }

    pub async fn save(&self, conn: &mut crate::DbConn) -> Result<usize, diesel::result::Error> {
        match diesel::replace_into(ticket_schema_reviews::table)
            .values(self)
            .execute(conn)
            .await
        {
            Ok(result) => Ok(result),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                _,
            )) => {
                diesel::update(ticket_schema_reviews::table)
                    .filter(ticket_schema_reviews::id.eq(&self.id))
                    .set(self)
                    .execute(conn)
                    .await
            }
            Err(e) => Err(e),
        }
    }
}

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
    Insertable,
)]
#[diesel(belongs_to(TicketFlow))]
#[diesel(belongs_to(TicketSchemaReview))]
#[diesel(table_name = ticket_reviews)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketReview {
    pub id: i32,
    pub ticket_flow_id: i32,
    pub ticket_schema_review_id: i32,
    pub approved: bool,
    pub comment: Option<String>,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketReview {
    pub async fn save_or_create(
        conn: &mut crate::DbConn,
        ticket_flow: &TicketFlow,
        ticket_schema_review: &TicketSchemaReview,
        approved: bool,
        comment: Option<String>,
    ) -> Result<TicketReview, diesel::result::Error> {
        let ticket_review: Result<TicketReview, _> = ticket_reviews::table
            .filter(ticket_reviews::ticket_flow_id.eq(ticket_flow.id))
            .filter(ticket_reviews::ticket_schema_review_id.eq(ticket_schema_review.id))
            .first(conn)
            .await;

        match ticket_review {
            Ok(mut review) => {
                review.approved = approved;
                review.comment = comment;
                review.save(conn).await?;
                Ok(review)
            }
            Err(diesel::result::Error::NotFound) => {
                diesel::insert_into(ticket_reviews::table)
                    .values((
                        ticket_reviews::ticket_flow_id.eq(ticket_flow.id),
                        ticket_reviews::ticket_schema_review_id.eq(ticket_schema_review.id),
                        ticket_reviews::approved.eq(approved),
                        ticket_reviews::comment.eq(comment),
                    ))
                    .execute(conn)
                    .await?;

                sql_function! {
                    fn last_insert_id() -> Integer;
                }

                ticket_reviews::table
                    .find(last_insert_id())
                    .first(conn)
                    .await
            }
            Err(e) => Err(e),
        }
    }

    pub async fn save(&self, conn: &mut crate::DbConn) -> Result<usize, diesel::result::Error> {
        match diesel::replace_into(ticket_reviews::table)
            .values(self)
            .execute(conn)
            .await
        {
            Ok(result) => Ok(result),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                _,
            )) => {
                diesel::update(ticket_reviews::table)
                    .filter(ticket_reviews::id.eq(&self.id))
                    .set(self)
                    .execute(conn)
                    .await
            }
            Err(e) => Err(e),
        }
    }
}
