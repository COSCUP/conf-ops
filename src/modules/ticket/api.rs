use std::collections::HashMap;
use std::iter;

use rocket::form::Form;
use rocket::fs::NamedFile;
use rocket::fs::TempFile;
use rocket::serde::json::serde_json;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use rocket::Route;
use rocket::State;
use rocket_db_pools::diesel::scoped_futures::ScopedFutureExt;
use rocket_db_pools::diesel::AsyncConnection;
use serde_json::Map;

use super::forms::fields::FormFieldDefine;
use super::forms::fields::FormSchemaField;
use super::forms::models::TicketFormAnswer;
use super::forms::models::TicketFormFile;
use super::forms::models::TicketFormImage;
use super::forms::models::TicketSchemaForm;
use super::models::TicketSchema;
use super::models::TicketSchemaFlow;
use super::reviews::models::TicketReview;
use super::reviews::models::TicketSchemaReview;
use super::TicketFlowItem;
use super::TicketFlowStatus;
use super::TicketFlowValue;
use super::TicketSchemaFlowItem;
use super::TicketSchemaFlowValue;
use super::TicketStatus;
use super::TicketWithStatus;

use crate::error::AppError;
use crate::models::target::Target;
use crate::models::user::User;
use crate::modules::ticket::models::Ticket;
use crate::modules::ApiResult;
use crate::modules::{guard::AuthGuard, EmptyResponse, EmptyResult, JsonResult};
use crate::utils::i18n::I18n;
use crate::DataFolder;
use crate::DbConn;

#[get("/ticket/tickets")]
async fn all_tickets(mut conn: DbConn, auth: AuthGuard) -> JsonResult<Vec<TicketWithStatus>> {
    let AuthGuard { user, .. } = auth;
    let tickets = Ticket::get_tickets_by_user(&mut conn, &user)
        .await
        .map_or(Json(vec![]), Json);
    Ok(tickets)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketDetail {
    #[serde(flatten)]
    pub ticket: TicketWithStatus,
    pub schema: TicketSchema,
    pub flows: Vec<TicketFlowStatus>,
}

#[get("/ticket/tickets/<ticket_id>")]
async fn get_ticket<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    ticket_id: i32,
) -> JsonResult<TicketDetail> {
    let AuthGuard { user, .. } = auth;
    let ticket = Ticket::find(&mut conn, ticket_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match ticket.is_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_join_to_this_ticket"),
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
        .get_schema(&mut conn)
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

    let ticket_status = match ticket.finished {
        true => TicketStatus::Finished,
        false => {
            let process_flow = flows.iter().find(|f| {
                if let Some(flow) = &f.flow {
                    flow.flow.finished == false
                } else {
                    false
                }
            });

            match process_flow {
                Some(flow) => match flow.flow.as_ref().and_then(|f| f.flow.user_id.clone()) {
                    Some(flow_user_id) if flow_user_id == user.id => TicketStatus::Pending,
                    Some(_) => TicketStatus::InProgress,
                    None => {
                        let target = Target::find(&mut conn, flow.schema.schema.operator_id)
                            .await
                            .map_err(|err| AppError::internal(err.to_string()))?;
                        let is_user = Target::is_user_in_targets(&mut conn, &user, &vec![target])
                            .await
                            .map_err(|err| AppError::internal(err.to_string()))?;

                        match is_user {
                            true => TicketStatus::Pending,
                            false => TicketStatus::InProgress,
                        }
                    }
                },
                None => TicketStatus::Finished,
            }
        }
    };

    Ok(Json(TicketDetail {
        ticket: TicketWithStatus {
            ticket,
            status: ticket_status,
        },
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
async fn process_ticket_flow<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    ticket_id: i32,
    flow_req: Json<TicketFlowProcessReq>,
) -> EmptyResult {
    let AuthGuard { user, .. } = auth;
    let mut ticket = Ticket::find(&mut conn, ticket_id)
        .await
        .map_err(|err| crate::error::AppError::not_found(err.to_string()))?;
    match ticket.is_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_join_to_this_ticket"),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }
    ticket.updated_at = chrono::Utc::now().naive_utc();

    let mut process_flow = ticket
        .get_process_flow(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    if let Some(flow_user_id) = &process_flow.user_id {
        if flow_user_id != &user.id {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_assign_to_this_flow"),
            ));
        }
    }

    let process_schema = process_flow
        .get_schema(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let schema_target = Target::find(&mut conn, process_schema.schema.operator_id)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;
    let is_schema_user = Target::is_user_in_targets(&mut conn, &user, &vec![schema_target])
        .await
        .map_err(|err| AppError::forbidden(err.to_string()))?;
    if !is_schema_user {
        return Err(AppError::forbidden(
            i18n.t("ticket.error.not_assign_to_this_schema"),
        ));
    }

    let latest_flow = ticket
        .get_latest_flow(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let req = flow_req.into_inner();

    match process_schema.module {
        TicketSchemaFlowValue::Form(form_schema) => {
            if let TicketProcessFlow::Form(form_data) = req.flow {
                match form_schema
                    .validate_and_normalize(&mut conn, &i18n, &form_data)
                    .await
                {
                    Ok(normalized_data) => {
                        conn.transaction(|conn| {
                            async move {
                                let _ = ticket.save(conn).await?;
                                process_flow.user_id = Some(user.id.clone());
                                process_flow.finished = true;
                                let _ = process_flow.save(conn).await?;
                                let _ = TicketFormAnswer::save_or_create(
                                    conn,
                                    &process_flow,
                                    &form_schema,
                                    normalized_data,
                                )
                                .await?;

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
                        return Err(AppError::bad_request_with_fields(i18n, fields));
                    }
                }
            }
            Err(AppError::bad_request("Invalid request".to_owned()))
        }
        TicketSchemaFlowValue::Review(review_schema) => {
            if let TicketProcessFlow::Review(review_req) = req.flow {
                conn.transaction(|conn| {
                    async move {
                        let _ = ticket.save(conn).await?;
                        let _ = TicketReview::save_or_create(
                            conn,
                            &process_flow,
                            &review_schema,
                            review_req.approved,
                            review_req.comment,
                        )
                        .await?;
                        process_flow.user_id = Some(user.id.clone());

                        if review_req.approved {
                            process_flow.finished = true;
                            let _ = process_flow.save(conn).await?;

                            if latest_flow.id == process_flow.id {
                                let _ = ticket.set_finish(conn, true).await?;
                            }
                        } else if review_schema.restarted {
                            let _ = process_flow.save(conn).await?;
                            let flows = ticket.get_flows(conn).await?;
                            for mut flow in flows.into_iter() {
                                flow.flow.finished = false;
                                let _ = flow.flow.save(conn).await?;
                            }
                        } else {
                            let _ = process_flow.save(conn).await?;
                            let mut previous_flow =
                                ticket.get_previous_flow(conn, &process_flow).await?;
                            previous_flow.finished = false;
                            let _ = previous_flow.save(conn).await?;
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
async fn all_probably_schemas(mut conn: DbConn, auth: AuthGuard) -> JsonResult<Vec<TicketSchema>> {
    let AuthGuard { user, .. } = auth;
    Ok(TicketSchema::get_probably_schemas(&mut conn, &user)
        .await
        .map_or(Json(vec![]), Json))
}

#[get("/ticket/schemas/<schema_id>")]
async fn get_schema<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    schema_id: i32,
) -> JsonResult<TicketSchemaDetail> {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_probably_join_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_probably_user_of_this_schema"),
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

#[get("/ticket/schemas/<schema_id>/flows/<flow_id>/probably_assign_users")]
async fn get_probably_assign_user_in_schema_flow<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    schema_id: i32,
    flow_id: i32,
) -> JsonResult<Vec<User>> {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_probably_join_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_probably_user_of_this_schema"),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    let schema_flow = TicketSchemaFlow::find(&mut conn, flow_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    let users = schema_flow
        .get_probably_assign_users(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;
    Ok(Json(users))
}

#[derive(Serialize, Deserialize, Debug)]
struct AddTicketReq {
    pub title: String,
    pub assign_flow_users: HashMap<i32, String>,
}

#[post("/ticket/schemas/<schema_id>/tickets", data = "<new_ticket_req>")]
async fn add_ticket_for_schema<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    schema_id: i32,
    new_ticket_req: Json<AddTicketReq>,
) -> EmptyResult {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    match schema.is_probably_join_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_join_to_this_ticket"),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    let flows = schema
        .get_flows(&mut conn)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    let AddTicketReq {
        title,
        assign_flow_users,
    } = new_ticket_req.into_inner();

    conn.transaction(|conn| {
        async move {
            let ticket = Ticket::create(conn, &schema, &title).await?;

            let _ = ticket.fill_flows(conn, &flows, assign_flow_users).await?;

            Ok::<_, diesel::result::Error>(EmptyResponse)
        }
        .scope_boxed()
    })
    .await
    .map_err(|err| AppError::internal(err.to_string()))?;
    Ok(EmptyResponse)
}

#[derive(FromForm)]
struct UploadFormField<'r> {
    file: TempFile<'r>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum UploadResult {
    File(TicketFormFile),
    Image(TicketFormImage),
}

#[post(
    "/ticket/schemas/<schema_id>/form/<form_id>/field/<field_id>/upload",
    data = "<upload_file_req>"
)]
async fn upload_file_in_form_field<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    data_folder: &State<DataFolder>,
    schema_id: i32,
    form_id: i32,
    field_id: i32,
    upload_file_req: Form<UploadFormField<'_>>,
) -> JsonResult<UploadResult> {
    let AuthGuard { user, .. } = auth;

    let UploadFormField { file } = upload_file_req.into_inner();

    let form = TicketSchemaForm::find_with_field(&mut conn, form_id, field_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    let schema_flow = TicketSchemaFlow::find(&mut conn, form.form.ticket_schema_flow_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    if schema_flow.ticket_schema_id != schema_id {
        return Err(AppError::bad_request("Invalid schema".to_owned()));
    }
    let schema_target = Target::find(&mut conn, schema_flow.operator_id)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;
    let is_schema_user = Target::is_user_in_targets(&mut conn, &user, &vec![schema_target])
        .await
        .map_err(|err| AppError::forbidden(err.to_string()))?;
    if !is_schema_user {
        return Err(AppError::forbidden(
            i18n.t("ticket.error.not_assign_to_this_schema"),
        ));
    }

    if form.field.is_file_define() {
        return form
            .upload_file(&mut conn, data_folder, &i18n, file)
            .await
            .map(|mut f: TicketFormFile| {
                f.path = String::new();
                Json(UploadResult::File(f))
            })
            .map_err(|err| AppError::bad_request(err.to_string()));
    }

    if form.field.is_image_define() {
        return form
            .upload_image(&mut conn, data_folder, &i18n, file)
            .await
            .map(|mut f| {
                f.path = String::new();
                Json(UploadResult::Image(f))
            })
            .map_err(|err| AppError::bad_request(err.to_string()));
    }

    Err(AppError::bad_request("Invalid request".to_owned()))
}

#[get("/ticket/images/<image_id>")]
async fn get_public_image_content(
    mut conn: DbConn,
    image_id: String
) -> ApiResult<NamedFile> {
    let image = TicketFormImage::find(&mut conn, image_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    NamedFile::open(image.path)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))
}

#[get("/ticket/schemas/<schema_id>/form/<form_id>/field/<field_id>/<file_id>")]
async fn get_field_file_content<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    schema_id: i32,
    form_id: i32,
    field_id: i32,
    file_id: String,
) -> ApiResult<NamedFile> {
    let AuthGuard { user, .. } = auth;

    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    match schema.is_probably_user(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_join_to_this_ticket"),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    let form = TicketSchemaForm::find_with_field(&mut conn, form_id, field_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;

    let mut id = file_id.clone();
    if file_id.contains('.') {
        id = file_id.split('.').collect::<Vec<&str>>()[0].to_owned();
    }

    let file_path = match form.field.define {
        FormFieldDefine::File { .. } => TicketFormFile::find(&mut conn, id)
            .await
            .map(|f| f.path)
            .map_err(|err| AppError::not_found(err.to_string()))?,
        FormFieldDefine::Image { .. } => TicketFormImage::find(&mut conn, id)
            .await
            .map(|f| f.path)
            .map_err(|err| AppError::not_found(err.to_string()))?,
        _ => return Err(AppError::not_found("Not Found".to_owned())),
    };

    NamedFile::open(file_path)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))
}

#[get("/ticket/admin/schemas")]
async fn all_managed_schemas_in_admin(
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
async fn get_managed_schema_in_admin<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    schema_id: i32,
) -> JsonResult<TicketSchemaDetail> {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_manager_of_this_schema"),
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
struct NewTicketSchemaReq {
    pub title_zh: String,
    pub title_en: String,
    pub description_zh: String,
    pub description_en: String,
}

#[post("/ticket/admin/schemas", data = "<new_schema_req>")]
async fn add_managed_schema_in_admin(
    mut conn: DbConn,
    auth: AuthGuard,
    new_schema_req: Json<NewTicketSchemaReq>,
) -> EmptyResult {
    let AuthGuard { user, project, .. } = auth;
    conn.transaction(|conn| {
        async move {
            let schema = TicketSchema::create(
                conn,
                new_schema_req.title_zh.clone(),
                new_schema_req.title_en.clone(),
                new_schema_req.description_zh.clone(),
                new_schema_req.description_en.clone(),
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
async fn add_flow_to_schema_in_admin<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    schema_id: i32,
    new_flow_req: Json<TicketSchemaFlowItem>,
) -> EmptyResult {
    let AuthGuard { user, .. } = auth;
    let mut schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_manager_of_this_schema"),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    };

    conn.transaction(|conn| {
        async move {
            schema.updated_at = chrono::Utc::now().naive_utc();
            let _ = schema.save(conn).await?;
            let flow = schema
                .add_flow(
                    conn,
                    new_flow_req.schema.name_zh.clone(),
                    new_flow_req.schema.name_en.clone(),
                )
                .await?;

            match new_flow_req.into_inner().module {
                TicketSchemaFlowValue::Form(form_schema) => {
                    let schema_form =
                        TicketSchemaForm::create(conn, &flow, form_schema.form.expired_at).await?;
                    let fields = form_schema
                        .fields
                        .into_iter()
                        .map(|field| FormSchemaField {
                            name_zh: field.name_zh,
                            description_zh: field.description_zh,
                            name_en: field.name_en,
                            description_en: field.description_en,
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
async fn all_tickets_for_schema_in_admin<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    schema_id: i32,
) -> JsonResult<Vec<Ticket>> {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_manager_of_this_schema"),
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTicketData {
    pub fields: Vec<Value>,
    pub data: Vec<Map<String, Value>>
}

#[get("/ticket/admin/schemas/<schema_id>/exports")]
async fn export_tickets_for_schema_in_admin<'a>(
    mut conn: DbConn,
    auth: AuthGuard,
    i18n: I18n<'a>,
    schema_id: i32,
) -> JsonResult<ExportTicketData> {
    let AuthGuard { user, .. } = auth;
    let schema = TicketSchema::find(&mut conn, schema_id)
        .await
        .map_err(|err| AppError::not_found(err.to_string()))?;
    match schema.is_manager(&mut conn, &user).await {
        Ok(false) => {
            return Err(AppError::forbidden(
                i18n.t("ticket.error.not_manager_of_this_schema"),
            ))
        }
        Err(err) => return Err(AppError::forbidden(err.to_string())),
        _ => (),
    }

    let tickets = schema
        .get_tickets(&mut conn)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let mut response = ExportTicketData {
        fields: vec![],
        data: vec![]
    };

    let schema_flows = schema.get_detail_flows(&mut conn).await.map_err(|err| AppError::internal(err.to_string()))?;

    response.fields.push(serde_json::json!({
        "module_type": "Base",
        "key": "ticket_id"
    }));

    for schema_flow in &schema_flows {
        match &schema_flow.module {
            TicketSchemaFlowValue::Form(form) => {
                for form_field in &form.fields {
                    if form_field.key == "" {
                        continue;
                    };
                    response.fields.push(serde_json::json!({
                        "module_type": "Form",
                        "key": form_field.key,
                        "define": form_field.define,
                        "name_zh": form_field.name_zh,
                        "name_en": form_field.name_en,
                    }));
                }
            },
            TicketSchemaFlowValue::Review(_) => {
                let review_key_prefix = format!("review_{}", schema_flow.schema.order);
                response.fields.push(serde_json::json!({
                    "module_type": "Review",
                    "key": format!("{}_approved", review_key_prefix)
                }));
                response.fields.push(serde_json::json!({
                    "module_type": "Review",
                    "key": format!("{}_comment", review_key_prefix)
                }));
            }
        }
    }

    for ticket in tickets {
        let mut ticket_data = Map::new();
        ticket_data.insert("ticket_id".to_owned(), serde_json::Value::Number(serde_json::Number::from(ticket.id)));
        for flow in ticket.get_flows(&mut conn).await.map_err(|err| AppError::internal(err.to_string()))? {
            let schema_flow = schema_flows.iter().find(|f| f.schema.id == flow.flow.ticket_schema_flow_id).expect("Schema flow not found");

            match flow.module {
                TicketFlowValue::Form(form) => {
                    if let serde_json::Value::Object(form_value) = form.value {
                        for (key, value) in form_value {
                            ticket_data.insert(key, value);
                        }
                    }
                },
                TicketFlowValue::Review(review) => {
                    let review_key_prefix = format!("review_{}", schema_flow.schema.order);
                    ticket_data.insert(format!("{}_approved", review_key_prefix), serde_json::Value::Bool(review.approved));
                    ticket_data.insert(format!("{}_comment", review_key_prefix), serde_json::Value::String(review.comment.unwrap_or("".to_owned())));
                },
                TicketFlowValue::None => ()
            }
        }
        response.data.push(ticket_data);
    }


    Ok(Json(response))
}

pub fn routes() -> Vec<Route> {
    routes![
        all_tickets,
        get_ticket,
        process_ticket_flow,
        all_probably_schemas,
        get_schema,
        get_probably_assign_user_in_schema_flow,
        add_ticket_for_schema,
        upload_file_in_form_field,
        get_field_file_content,
        get_public_image_content,
        all_managed_schemas_in_admin,
        get_managed_schema_in_admin,
        add_managed_schema_in_admin,
        add_flow_to_schema_in_admin,
        all_tickets_for_schema_in_admin,
        export_tickets_for_schema_in_admin,
    ]
}
