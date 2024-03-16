use std::collections::HashMap;

use chrono::NaiveDateTime;
use diesel::dsl::{max, min};
use diesel::prelude::*;
use rocket_db_pools::diesel::prelude::RunQueryDsl;

use crate::models::user::User;
use crate::models::{project::Project, target::Target};
use crate::schema::{
    labels, targets, ticket_flows, ticket_form_answers, ticket_reviews, ticket_schema_flows,
    ticket_schema_forms, ticket_schema_managers, ticket_schema_reviews, ticket_schemas, tickets,
};
use crate::utils::serde::unix_time;

use super::forms::models::{TicketFormAnswer, TicketSchemaForm, TicketSchemaFormField};
use super::forms::FormSchema;
use super::reviews::models::{TicketReview, TicketSchemaReview};
use super::{TicketFlowItem, TicketFlowValue, TicketSchemaFlowItem, TicketSchemaFlowValue};

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
#[diesel(table_name = ticket_schemas)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketSchema {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub project_id: String,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketSchema {
    pub async fn create(
        conn: &mut crate::DbConn,
        title: String,
        description: String,
        project_id: String,
    ) -> Result<TicketSchema, diesel::result::Error> {
        let _ = diesel::insert_into(ticket_schemas::table)
            .values((
                ticket_schemas::title.eq(title),
                ticket_schemas::description.eq(description),
                ticket_schemas::project_id.eq(project_id),
            ))
            .execute(conn)
            .await;

        sql_function! {
            fn last_insert_id() -> Integer;
        }

        Ok(ticket_schemas::table
            .find(last_insert_id())
            .first(conn)
            .await?)
    }

    pub async fn find(
        conn: &mut crate::DbConn,
        id: i32,
    ) -> Result<TicketSchema, diesel::result::Error> {
        ticket_schemas::table.find(id).first(conn).await
    }

    pub async fn save(&self, conn: &mut crate::DbConn) -> Result<usize, diesel::result::Error> {
        diesel::replace_into(ticket_schemas::table)
            .values(self)
            .execute(conn)
            .await
    }

    pub async fn get_managers(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<User>, diesel::result::Error> {
        let managers = TicketSchemaManager::belonging_to(self)
            .inner_join(targets::table)
            .select(Target::as_select())
            .load(conn)
            .await?;

        Target::get_users(conn, &managers).await
    }

    pub async fn get_manager_schemas(
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<Vec<TicketSchema>, diesel::result::Error> {
        let user_label_ids = user
            .get_labels_by_key(conn, "role".to_owned())
            .await?
            .iter()
            .map(|label| label.id.clone())
            .collect::<Vec<i32>>();

        ticket_schemas::table
            .inner_join(ticket_schema_managers::table.inner_join(targets::table.left_join(labels::table)))
            .filter(labels::id.eq_any(user_label_ids))
            .or_filter(targets::user_id.eq(user.id.clone()))
            .select(TicketSchema::as_select())
            .load(conn)
            .await
    }

    pub async fn is_manager(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<bool, diesel::result::Error> {
        let managers = TicketSchemaManager::belonging_to(self)
            .inner_join(targets::table)
            .select(Target::as_select())
            .load(conn)
            .await?;

        Target::is_user_in_targets(conn, user, &managers).await
    }

    pub async fn get_probably_schemas(
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<Vec<TicketSchema>, diesel::result::Error> {
        let user_label_ids = user
            .get_labels_by_key(conn, "role".to_owned())
            .await?
            .iter()
            .map(|label| label.id.clone())
            .collect::<Vec<i32>>();

        ticket_schemas::table
            .inner_join(
                ticket_schema_flows::table.inner_join(targets::table.left_join(labels::table)),
            )
            .filter(ticket_schema_flows::order.eq(0))
            .filter(labels::id.eq_any(user_label_ids))
            .or_filter(targets::user_id.eq(user.id.clone()))
            .select(TicketSchema::as_select())
            .load(conn)
            .await
    }

    pub async fn add_manager_user(
        &self,
        conn: &mut crate::DbConn,
        user: &User
    ) -> Result<TicketSchemaManager, diesel::result::Error> {
        let target = Target::find_or_create_user(conn, user).await?;

        let _ = diesel::insert_into(ticket_schema_managers::table)
            .values((
                ticket_schema_managers::ticket_schema_id.eq(self.id),
                ticket_schema_managers::target_id.eq(target.id),
            ))
            .execute(conn)
            .await;

        sql_function! {
            fn last_insert_id() -> Integer;
        }

        Ok(ticket_schema_managers::table
            .find(last_insert_id())
            .first(conn)
            .await?)
    }

    pub async fn is_probably_user(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<bool, diesel::result::Error> {
        let users: Vec<Target> = TicketSchemaFlow::belonging_to(self)
            .inner_join(targets::table)
            .filter(ticket_schema_flows::order.eq(0))
            .select(Target::as_select())
            .load(conn)
            .await?;

        Target::is_user_in_targets(conn, user, &users).await
    }

    pub async fn get_flows (
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<TicketSchemaFlow>, diesel::result::Error> {
        ticket_schema_flows::table
            .filter(ticket_schema_flows::ticket_schema_id.eq(self.id))
            .order(ticket_schema_flows::order.asc())
            .load(conn)
            .await
    }

    pub async fn get_detail_flows(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<TicketSchemaFlowItem>, diesel::result::Error> {
        let flows: Vec<(
            TicketSchemaFlow,
            Option<TicketSchemaForm>,
            Option<TicketSchemaReview>,
        )> = TicketSchemaFlow::belonging_to(self)
            .order(ticket_schema_flows::order.asc())
            .left_join(ticket_schema_forms::table)
            .left_join(ticket_schema_reviews::table)
            .select((
                TicketSchemaFlow::as_select(),
                Option::<TicketSchemaForm>::as_select(),
                Option::<TicketSchemaReview>::as_select(),
            ))
            .load(conn)
            .await?;

        let forms = flows
            .iter()
            .filter_map(|(_, form, _)| {
                if let Some(form) = form {
                    return Some(form);
                }
                return None;
            })
            .collect::<Vec<_>>();

        let fields: Vec<TicketSchemaFormField> = TicketSchemaFormField::belonging_to(&forms)
            .load(conn)
            .await?;

        let mut result: Vec<TicketSchemaFlowItem> = vec![];

        for (schema_flow, raw_form, raw_review) in flows.into_iter() {
            if let Some(form) = raw_form {
                let form_id = form.id;
                let form_schema = FormSchema::new(
                    form,
                    fields
                        .iter()
                        .cloned()
                        .filter(|field| field.ticket_schema_form_id == form_id)
                        .collect::<Vec<_>>(),
                );

                result.push(TicketSchemaFlowItem {
                    schema: schema_flow,
                    module: TicketSchemaFlowValue::Form(form_schema),
                });
                continue;
            }

            if let Some(review) = raw_review {
                result.push(TicketSchemaFlowItem {
                    schema: schema_flow,
                    module: TicketSchemaFlowValue::Review(review),
                });
                continue;
            }
        }

        Ok(result)
    }

    pub async fn get_detail_flows_with_user(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<Vec<TicketSchemaFlowItem>, diesel::result::Error> {
        let flows: Vec<(
            TicketSchemaFlow,
            Option<TicketSchemaForm>,
            Option<TicketSchemaReview>,
        )> = TicketSchemaFlow::belonging_to(self)
            .left_join(ticket_schema_forms::table)
            .left_join(ticket_schema_reviews::table)
            .select((
                TicketSchemaFlow::as_select(),
                Option::<TicketSchemaForm>::as_select(),
                Option::<TicketSchemaReview>::as_select(),
            ))
            .load(conn)
            .await?;

        let forms = flows
            .iter()
            .filter_map(|(_, form, _)| {
                if let Some(form) = form {
                    return Some(form);
                }
                return None;
            })
            .collect::<Vec<_>>();

        let fields: Vec<TicketSchemaFormField> = TicketSchemaFormField::belonging_to(&forms)
            .load(conn)
            .await?;

        let mut result: Vec<TicketSchemaFlowItem> = vec![];

        for (schema_flow, raw_form, raw_review) in flows.into_iter() {
            if let Some(form) = raw_form {
                let form_id = form.id;
                let form_schema = FormSchema::new_with_user(
                    conn,
                    user,
                    form,
                    fields
                        .iter()
                        .cloned()
                        .filter(|field| field.ticket_schema_form_id == form_id)
                        .collect::<Vec<_>>(),
                )
                .await;

                result.push(TicketSchemaFlowItem {
                    schema: schema_flow,
                    module: TicketSchemaFlowValue::Form(form_schema),
                });
                continue;
            }

            if let Some(review) = raw_review {
                result.push(TicketSchemaFlowItem {
                    schema: schema_flow,
                    module: TicketSchemaFlowValue::Review(review),
                });
                continue;
            }
        }

        Ok(result)
    }

    pub async fn add_flow(
        &self,
        conn: &mut crate::DbConn,
        name: String,
    ) -> Result<TicketSchemaFlow, diesel::result::Error> {
        let max_order: Option<i32> = ticket_schema_flows::table
            .filter(ticket_schema_flows::ticket_schema_id.eq(self.id))
            .select(max(ticket_schema_flows::order))
            .first::<Option<i32>>(conn)
            .await?;

        let order = match max_order {
            Some(order) => order + 1,
            None => 0,
        };

        let _ = diesel::insert_into(ticket_schema_flows::table)
            .values((
                ticket_schema_flows::ticket_schema_id.eq(self.id),
                ticket_schema_flows::order.eq(order),
                ticket_schema_flows::name.eq(name),
            ))
            .execute(conn)
            .await;

        sql_function! {
            fn last_insert_id() -> Integer;
        }

        let flow = ticket_schema_flows::table
            .find(last_insert_id())
            .first(conn)
            .await?;

        Ok(flow)
    }

    pub async fn get_tickets(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<Ticket>, diesel::result::Error> {
        tickets::table
            .filter(tickets::ticket_schema_id.eq(self.id))
            .load(conn)
            .await
    }
}

#[derive(
    Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(TicketSchema))]
#[diesel(belongs_to(Target))]
#[diesel(table_name = ticket_schema_managers)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketSchemaManager {
    pub id: i32,
    pub ticket_schema_id: i32,
    pub target_id: i32,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

#[derive(
    Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(TicketSchema))]
#[diesel(belongs_to(Target, foreign_key = operator_id))]
#[diesel(table_name = ticket_schema_flows)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketSchemaFlow {
    pub id: i32,
    pub ticket_schema_id: i32,
    pub operator_id: i32,
    pub order: i32,
    pub name: String,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketSchemaFlow {
    pub async fn find(
        conn: &mut crate::DbConn,
        id: i32,
    ) -> Result<TicketSchemaFlow, diesel::result::Error> {
        ticket_schema_flows::table.find(id).first(conn).await
    }

    pub async fn get_detail(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<TicketSchemaFlowItem, diesel::result::Error> {
        let (flow, form, review) = ticket_schema_flows::table
            .filter(ticket_schema_flows::id.eq(self.id))
            .left_join(ticket_schema_forms::table)
            .left_join(ticket_schema_reviews::table)
            .select((
                TicketSchemaFlow::as_select(),
                Option::<TicketSchemaForm>::as_select(),
                Option::<TicketSchemaReview>::as_select(),
            ))
            .first(conn)
            .await?;

        if let Some(form) = form {
            let form_id = form.id;
            let fields: Vec<TicketSchemaFormField> = TicketSchemaFormField::belonging_to(&form)
                .load(conn)
                .await?;

            let mut form_schema = FormSchema {
                form,
                fields: fields
                    .iter()
                    .cloned()
                    .filter(|field| field.ticket_schema_form_id == form_id)
                    .collect::<Vec<_>>(),
            };
            form_schema.fields.sort_by_key(|field| field.order);

            return Ok(TicketSchemaFlowItem {
                schema: flow,
                module: TicketSchemaFlowValue::Form(form_schema),
            });
        }

        if let Some(review) = review {
            return Ok(TicketSchemaFlowItem {
                schema: flow,
                module: TicketSchemaFlowValue::Review(review),
            });
        }

        Err(diesel::result::Error::NotFound)
    }
}

#[derive(
    Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(TicketSchema))]
#[diesel(table_name = tickets)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Ticket {
    pub id: i32,
    pub ticket_schema_id: i32,
    pub finished: bool,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl Ticket {
    pub async fn create(
        conn: &mut crate::DbConn,
        schema: &TicketSchema,
    ) -> Result<Ticket, diesel::result::Error> {
        let _ = diesel::insert_into(tickets::table)
            .values((
                tickets::ticket_schema_id.eq(schema.id),
                tickets::finished.eq(false),
            ))
            .execute(conn)
            .await;

        sql_function! {
            fn last_insert_id() -> Integer;
        }

        Ok(tickets::table.find(last_insert_id()).first(conn).await?)
    }

    pub async fn find(conn: &mut crate::DbConn, id: i32) -> Result<Ticket, diesel::result::Error> {
        tickets::table.find(id).first(conn).await
    }

    pub async fn set_finish(
        &self,
        conn: &mut crate::DbConn,
        finished: bool,
    ) -> Result<usize, diesel::result::Error> {
        diesel::update(tickets::table.filter(tickets::id.eq(self.id)))
            .set(tickets::finished.eq(finished))
            .execute(conn)
            .await
    }

    pub async fn get_pending_tickets_by_user(
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<Vec<Ticket>, diesel::result::Error> {
        let sub_query = ticket_flows::table
            .inner_join(ticket_schema_flows::table)
            .filter(ticket_flows::user_id.eq(user.id.clone()))
            .filter(ticket_flows::finished.eq(false))
            .group_by(ticket_flows::ticket_id)
            .having(min(ticket_schema_flows::created_at).is_not_null())
            .select(ticket_flows::ticket_id);


        tickets::table
            .filter(tickets::id.eq_any(sub_query))
            .select(Ticket::as_select())
            .load(conn)
            .await
    }

    pub async fn get_tickets_by_user(
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<Vec<Ticket>, diesel::result::Error> {
        let sub_query = ticket_flows::table
            .inner_join(ticket_schema_flows::table)
            .filter(ticket_flows::user_id.eq(user.id.clone()))
            .filter(ticket_flows::finished.eq(false))
            .group_by(ticket_flows::ticket_id)
            .select(ticket_flows::ticket_id);

        tickets::table
            .filter(tickets::id.eq_any(sub_query))
            .select(Ticket::as_select())
            .load(conn)
            .await
    }

    pub async fn is_user(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<bool, diesel::result::Error> {
        let flows: Vec<TicketFlow> = TicketFlow::belonging_to(self)
            .filter(ticket_flows::user_id.eq(user.id.clone()))
            .load(conn)
            .await?;

        if !flows.is_empty() {
            return Ok(true);
        }

        let schema = TicketSchema::find(conn, self.ticket_schema_id).await?;
        schema.is_probably_user(conn, user).await
    }

    pub async fn get_schema(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> Result<(TicketSchema, Vec<TicketSchemaFlowItem>), diesel::result::Error> {
        let schema = TicketSchema::find(conn, self.ticket_schema_id).await?;
        let flows = schema.get_detail_flows_with_user(conn, user).await?;

        Ok((schema, flows))
    }

    pub async fn fill_flows(
        &self,
        conn: &mut crate::DbConn,
        flows: &Vec<TicketSchemaFlow>,
        assign_flow_users: HashMap<i32, String>,
    ) -> Result<(), diesel::result::Error> {
        let records = (0..flows.len())
            .map(|i| {
                let flow = &flows[i];
                let user_id = assign_flow_users.get(&flow.id).cloned();
                (
                    ticket_flows::ticket_id.eq(self.id),
                    ticket_flows::ticket_schema_flow_id.eq(flow.id),
                    ticket_flows::user_id.eq(user_id),
                    ticket_flows::finished.eq(false),
                )
            })
            .collect::<Vec<_>>();

        diesel::insert_into(ticket_flows::table)
            .values(records)
            .execute(conn)
            .await?;

        Ok(())
    }

    pub async fn get_flows(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<Vec<TicketFlowItem>, diesel::result::Error> {
        let flows: Vec<(TicketFlow, Option<TicketFormAnswer>, Option<TicketReview>)> =
            TicketFlow::belonging_to(self)
                .inner_join(ticket_schema_flows::table)
                .left_join(ticket_form_answers::table)
                .left_join(ticket_reviews::table)
                .order(ticket_schema_flows::order.asc())
                .select((
                    TicketFlow::as_select(),
                    Option::<TicketFormAnswer>::as_select(),
                    Option::<TicketReview>::as_select(),
                ))
                .load(conn)
                .await?;

        flows
            .into_iter()
            .map(|(flow, form, review)| {
                if let Some(form) = form {
                    return Ok(TicketFlowItem {
                        flow,
                        module: TicketFlowValue::Form(form),
                    });
                }

                if let Some(review) = review {
                    return Ok(TicketFlowItem {
                        flow,
                        module: TicketFlowValue::Review(review),
                    });
                }

                Err(diesel::result::Error::NotFound)
            })
            .collect()
    }

    pub async fn get_process_flow(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<TicketFlow, diesel::result::Error> {
        ticket_flows::table
            .inner_join(ticket_schema_flows::table)
            .filter(ticket_flows::ticket_id.eq(self.id))
            .filter(ticket_flows::finished.eq(false))
            .order(ticket_schema_flows::order.asc())
            .select(TicketFlow::as_select())
            .first(conn)
            .await
    }

    pub async fn get_latest_flow(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<TicketFlow, diesel::result::Error> {
        ticket_flows::table
            .inner_join(ticket_schema_flows::table)
            .filter(ticket_flows::ticket_id.eq(self.id))
            .order(ticket_schema_flows::order.desc())
            .select(TicketFlow::as_select())
            .first(conn)
            .await
    }
}

#[derive(
    Queryable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(Ticket))]
#[diesel(belongs_to(TicketSchemaFlow))]
#[diesel(belongs_to(User))]
#[diesel(table_name = ticket_flows)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketFlow {
    pub id: i32,
    pub ticket_id: i32,
    pub user_id: Option<String>,
    pub ticket_schema_flow_id: i32,
    pub finished: bool,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketFlow {
    pub async fn create(
        conn: &mut crate::DbConn,
        ticket: &Ticket,
        ticket_schema_flow: &TicketSchemaFlow,
        user_id: Option<String>,
        finished: bool,
    ) -> Result<TicketFlow, diesel::result::Error> {
        let _ = diesel::insert_into(ticket_flows::table)
            .values((
                ticket_flows::ticket_id.eq(ticket.id),
                ticket_flows::ticket_schema_flow_id.eq(ticket_schema_flow.id),
                ticket_flows::user_id.eq(user_id),
                ticket_flows::finished.eq(finished),
            ))
            .execute(conn)
            .await;

        sql_function! {
            fn last_insert_id() -> Integer;
        }

        Ok(ticket_flows::table
            .find(last_insert_id())
            .first(conn)
            .await?)
    }

    pub async fn set_finish(
        &self,
        conn: &mut crate::DbConn,
        finished: bool,
    ) -> Result<usize, diesel::result::Error> {
        diesel::update(ticket_flows::table.filter(ticket_flows::id.eq(self.id)))
            .set(ticket_flows::finished.eq(finished))
            .execute(conn)
            .await
    }

    pub async fn get_schema(
        &self,
        conn: &mut crate::DbConn,
    ) -> Result<TicketSchemaFlowItem, diesel::result::Error> {
        let schema_flow = TicketSchemaFlow::find(conn, self.ticket_schema_flow_id).await?;

        schema_flow.get_detail(conn).await
    }
}
