use std::collections::HashMap;
use std::iter;

use rocket::serde::json::serde_json;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use rocket::Route;
use rocket_db_pools::diesel::scoped_futures::ScopedFutureExt;
use rocket_db_pools::diesel::AsyncConnection;

use super::forms::fields::FormSchemaField;
use super::forms::models::TicketFormAnswer;
use super::forms::models::TicketSchemaForm;
use super::models::TicketSchema;
use super::reviews::models::TicketReview;
use super::reviews::models::TicketSchemaReview;
use super::TicketFlowItem;
use super::TicketFlowStatus;
use super::TicketSchemaFlowItem;
use super::TicketSchemaFlowValue;

use crate::error::AppError;
use crate::modules::ticket::models::Ticket;
use crate::modules::{AuthGuard, EmptyResponse, EmptyResult, JsonResult};
use crate::DbConn;

#[get("/ticket/tickets")]
pub async fn all_tickets(mut conn: DbConn, auth: AuthGuard) -> JsonResult<Vec<Ticket>> {
    let AuthGuard { user, .. } = auth;
    let tickets = Ticket::get_tickets_by_user(&mut conn, &user)
        .await
        .map_or(Json(vec![]), Json);
    Ok(tickets)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketDetail {
    #[serde(flatten)]
    pub ticket: Ticket,
    pub schema: TicketSchema,
    pub flows: Vec<TicketFlowStatus>,
}

#[get("/ticket/tickets/<ticket_id>")]
pub async fn get_ticket(
    mut conn: DbConn,
    auth: AuthGuard,
    ticket_id: i32,
) -> JsonResult<TicketDetail> {
    let AuthGuard { user, .. } = auth;
    let ticket = Ticket::find(&mut conn, ticket_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match ticket.is_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not join to this ticket".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    let flows = ticket
        .get_flows(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;
    let (schema, schema_flows) = ticket
        .get_schema(&mut conn, &user)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let mut status_flows: Vec<Option<TicketFlowItem>> =
        flows.into_iter().map(|flow| Some(flow)).collect::<Vec<_>>();
    status_flows.extend(
        iter::repeat_with(|| None::<TicketFlowItem>)
            .take(schema_flows.len().saturating_sub(status_flows.len())),
    );

    let flows = schema_flows
        .into_iter()
        .zip(status_flows.into_iter())
        .map(|(schema_flow, flow)| TicketFlowStatus {
            schema: schema_flow,
            flow,
        })
        .collect::<Vec<_>>();

    Ok(Json(TicketDetail {
        ticket,
        schema,
        flows,
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketFlowReviewReq {
    pub approved: bool,
    pub comment: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum TicketProcessFlow {
    Form(serde_json::Map<String, Value>),
    Review(TicketFlowReviewReq),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketFlowProcessReq {
    pub flow: TicketProcessFlow,
}

#[post("/ticket/tickets/<ticket_id>/process", data = "<flow_req>")]
pub async fn process_ticket_flow(
    mut conn: DbConn,
    auth: AuthGuard,
    ticket_id: i32,
    flow_req: Json<TicketFlowProcessReq>,
) -> EmptyResult {
    let AuthGuard { user, .. } = auth;
    let ticket = Ticket::find(&mut conn, ticket_id)
        .await
        .map_err(|err| crate::error::AppError::not_found(err.to_string()))?;
    match ticket.is_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not join to this ticket".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    let process_flow = ticket
        .get_process_flow(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let process_schema = process_flow
        .get_schema(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let latest_flow = ticket
        .get_latest_flow(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let req = flow_req.into_inner();

    match process_schema.module {
        TicketSchemaFlowValue::Form(form_schema) => {
            if let TicketProcessFlow::Form(form_data) = req.flow {
                match form_schema
                    .validate_and_normalize(&mut conn, &form_data)
                    .await
                {
                    Ok(normalized_data) => {
                        conn.transaction(|conn| {
                            async move {
                                let _ = TicketFormAnswer::create(
                                    conn,
                                    &process_flow,
                                    &form_schema,
                                    normalized_data,
                                )
                                .await?;
                                let _ = process_flow.set_finish(conn, true).await?;

                                if latest_flow.id == process_flow.id {
                                    let _ = ticket.set_finish(conn, true).await?;
                                }

                                Ok::<_, diesel::result::Error>(EmptyResponse)
                            }
                            .scope_boxed()
                        })
                        .await
                        .map_err(|err| AppError::internal(err.to_string()))?;
                        return Ok(EmptyResponse);
                    }
                    Err(fields) => {
                        return Err(AppError::bad_request_with_fields(fields));
                    }
                }
            }
            Err(AppError::bad_request("Invalid request".to_owned()))
        }
        TicketSchemaFlowValue::Review(review_schema) => {
            if let TicketProcessFlow::Review(review_req) = req.flow {
                conn.transaction(|conn| {
                    async move {
                        let _ = TicketReview::create(
                            conn,
                            &process_flow,
                            &review_schema,
                            review_req.approved,
                            review_req.comment,
                        )
                        .await?;
                        let _ = process_flow.set_finish(conn, true).await?;

                        if latest_flow.id == process_flow.id {
                            let _ = ticket.set_finish(conn, true).await?;
                        }

                        Ok::<_, diesel::result::Error>(())
                    }
                    .scope_boxed()
                })
                .await
                .map_err(|err| AppError::internal(err.to_string()))?;
                return Ok(EmptyResponse);
            }
            Err(AppError::bad_request("Invalid request".to_owned()))
        }
    }
}

#[get("/ticket/schemas")]
pub async fn all_probably_schemas(
    mut conn: DbConn,
    auth: AuthGuard,
) -> JsonResult<Vec<TicketSchema>> {
    let AuthGuard { user, .. } = auth;
    Ok(TicketSchema::get_probably_schemas(&mut conn, &user)
        .await
        .map_or(Json(vec![]), Json))
}

#[get("/ticket/schemas/<schema_id>")]
pub async fn get_schema(
    mut conn: DbConn,
    auth: AuthGuard,
    schema_id: i32,
) -> JsonResult<TicketSchemaDetail> {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_probably_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not a manager of this schema".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }
    let flows = schema
        .get_detail_flows(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    Ok(Json(TicketSchemaDetail { schema, flows }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewTicketReq {
    pub assign_flow_users: HashMap<i32, String>,
}

#[post("/ticket/schemas/<schema_id>/tickets", data = "<new_ticket_req>")]
pub async fn add_ticket_for_schema(
    mut conn: DbConn,
    auth: AuthGuard,
    schema_id: i32,
    new_ticket_req: Json<NewTicketReq>,
) -> EmptyResult {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    match schema.is_probably_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not join to this ticket".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    let flows = schema
        .get_flows(&mut conn)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    let assign_flow_users = new_ticket_req.into_inner().assign_flow_users;

    conn.transaction(|conn| {
        async move {
            let ticket = Ticket::create(conn, &schema).await?;

            let _ = ticket.fill_flows(conn, &flows, assign_flow_users).await?;

            Ok::<_, diesel::result::Error>(EmptyResponse)
        }
        .scope_boxed()
    })
    .await
    .map_err(|err| AppError::internal(err.to_string()))?;
    Ok(EmptyResponse)
}

#[get("/ticket/admin/schemas")]
pub async fn all_managed_schemas_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
) -> JsonResult<Vec<TicketSchema>> {
    let AuthGuard { user, .. } = auth;
    Ok(TicketSchema::get_manager_schemas(&mut conn, &user)
        .await
        .map_or(Json(vec![]), Json))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketSchemaDetail {
    #[serde(flatten)]
    pub schema: TicketSchema,
    pub flows: Vec<TicketSchemaFlowItem>,
}

#[get("/ticket/admin/schemas/<schema_id>")]
pub async fn get_managed_schema_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    schema_id: i32,
) -> JsonResult<TicketSchemaDetail> {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not a manager of this schema".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }
    let flows = schema
        .get_detail_flows(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    Ok(Json(TicketSchemaDetail { schema, flows }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewTicketSchemaReq {
    pub title: String,
    pub description: String,
}

#[post("/ticket/admin/schemas", data = "<new_schema_req>")]
pub async fn add_managed_schema_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    new_schema_req: Json<NewTicketSchemaReq>,
) -> EmptyResult {
    let AuthGuard { user, project, .. } = auth;
    conn.transaction(|conn| {
        async move {
            let schema = TicketSchema::create(
                conn,
                new_schema_req.title.clone(),
                new_schema_req.description.clone(),
                project.id,
            )
            .await?;

            let _ = schema.add_manager_user(conn, &user).await?;

            Ok::<_, diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await
    .map_err(|err| AppError::internal(err.to_string()))?;

    Ok(EmptyResponse)
}

#[post("/ticket/admin/schemas/<schema_id>/flows", data = "<new_flow_req>")]
pub async fn add_flow_to_schema_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    schema_id: i32,
    new_flow_req: Json<TicketSchemaFlowItem>,
) -> EmptyResult {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not a manager of this schema".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    };

    conn.transaction(|conn| {
        async move {
            let flow = schema
                .add_flow(conn, new_flow_req.schema.name.clone())
                .await?;

            match new_flow_req.into_inner().module {
                TicketSchemaFlowValue::Form(form_schema) => {
                    let schema_form =
                        TicketSchemaForm::create(conn, &flow, form_schema.form.expired_at).await?;
                    let fields = form_schema
                        .fields
                        .into_iter()
                        .map(|field| FormSchemaField {
                            key: field.key,
                            define: field.define,
                            required: field.required,
                            editable: field.editable,
                        })
                        .collect::<Vec<_>>();
                    schema_form.add_fields(conn, fields).await?;
                }
                TicketSchemaFlowValue::Review(review_schema) => {
                    TicketSchemaReview::create(conn, &flow, review_schema.restarted).await?;
                }
            }

            Ok::<_, diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await
    .map_err(|err| AppError::internal(err.to_string()))?;

    Ok(EmptyResponse)
}

#[get("/ticket/admin/schemas/<schema_id>/tickets")]
pub async fn all_tickets_for_schema_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    schema_id: i32,
) -> JsonResult<Vec<Ticket>> {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                "You are not a manager of this schema".to_owned(),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }
    let tickets = schema
        .get_tickets(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;
    Ok(Json(tickets))
}

pub fn routes() -> Vec<Route> {
    routes![
        all_tickets,
        get_ticket,
        process_ticket_flow,
        all_probably_schemas,
        get_schema,
        add_ticket_for_schema,
        all_managed_schemas_in_admin,
        get_managed_schema_in_admin,
        add_managed_schema_in_admin,
        add_flow_to_schema_in_admin,
        all_tickets_for_schema_in_admin,
    ]
}
