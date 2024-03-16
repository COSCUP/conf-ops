use chrono::NaiveDateTime;
use diesel::dsl::max;
use diesel::prelude::*;
use rocket::serde::json::serde_json;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use serde_json::Value;

use crate::models::user::User;
use crate::modules::ticket::models::{TicketFlow, TicketSchemaFlow};
use crate::schema::{
    ticket_flows, ticket_form_answers, ticket_form_files, ticket_form_images,
    ticket_schema_form_fields, ticket_schema_forms,
};
use crate::utils::file::FileMime;
use crate::utils::image::ImageMime;
use crate::utils::{
    serde::{unix_time, unix_time_option},
    string::StringExt,
};

use super::fields::{
    FormFieldDefault, FormFieldDefine, FormFieldOptionValue, FormFieldValue, FormSchemaField,
};
use super::{FormSchema, PartFormSchema};

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
#[diesel(belongs_to(TicketSchemaFlow))]
#[diesel(table_name = ticket_schema_forms)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketSchemaForm {
    pub id: i32,
    pub ticket_schema_flow_id: i32,
    #[serde(with = "unix_time_option")]
    pub expired_at: Option<NaiveDateTime>,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketSchemaForm {
    pub async fn create(
        conn: &mut crate::DbConn,
        schema_flow: &TicketSchemaFlow,
        expired_at: Option<NaiveDateTime>,
    ) -> Result<TicketSchemaForm, diesel::result::Error> {
        diesel::insert_into(ticket_schema_forms::table)
            .values((
                ticket_schema_forms::ticket_schema_flow_id.eq(schema_flow.id),
                ticket_schema_forms::expired_at.eq(expired_at),
            ))
            .execute(conn)
            .await?;

        sql_function! {
            fn last_insert_id() -> Integer;
        }

        ticket_schema_forms::table
            .find(last_insert_id())
            .first(conn)
            .await
    }

    pub async fn find(
        conn: &mut crate::DbConn,
        id: i32,
    ) -> Result<FormSchema, diesel::result::Error> {
        let form: TicketSchemaForm = ticket_schema_forms::table.find(id).first(conn).await?;
        let fields: Vec<_> = TicketSchemaFormField::belonging_to(&form)
            .select(TicketSchemaFormField::as_select())
            .load(conn)
            .await?;

        Ok(FormSchema { form, fields })
    }

    pub async fn find_with_field(
        conn: &mut crate::DbConn,
        id: i32,
        field: i32,
    ) -> Result<PartFormSchema, diesel::result::Error> {
        let (form, field): (TicketSchemaForm, TicketSchemaFormField) = ticket_schema_forms::table
            .find(id)
            .inner_join(ticket_schema_form_fields::table)
            .filter(ticket_schema_form_fields::id.eq(field))
            .select((
                TicketSchemaForm::as_select(),
                TicketSchemaFormField::as_select(),
            ))
            .first(conn)
            .await?;

        Ok(PartFormSchema { form, field })
    }

    pub async fn save(
        &self,
        conn: &mut crate::DbConn,
        fields: Vec<TicketSchemaFormField>,
    ) -> Result<usize, diesel::result::Error> {
        let _ = diesel::replace_into(ticket_schema_forms::table)
            .values(self)
            .execute(conn)
            .await?;

        diesel::replace_into(ticket_schema_form_fields::table)
            .values(fields)
            .execute(conn)
            .await
    }

    pub async fn add_fields(
        &self,
        conn: &mut crate::DbConn,
        fields: Vec<FormSchemaField>,
    ) -> Result<usize, diesel::result::Error> {
        let max_order: Option<i32> = ticket_schema_form_fields::table
            .filter(ticket_schema_form_fields::ticket_schema_form_id.eq(self.id))
            .select(max(ticket_schema_form_fields::order))
            .first::<Option<i32>>(conn)
            .await?;

        let order = match max_order {
            Some(order) => order + 1,
            None => 0,
        };

        let records = fields
            .into_iter()
            .enumerate()
            .map(|(i, field)| {
                (
                    ticket_schema_form_fields::ticket_schema_form_id.eq(self.id),
                    ticket_schema_form_fields::order.eq(order + i as i32),
                    ticket_schema_form_fields::key.eq(field.key),
                    ticket_schema_form_fields::define.eq(field.define),
                    ticket_schema_form_fields::required.eq(field.required),
                    ticket_schema_form_fields::editable.eq(field.editable),
                )
            })
            .collect::<Vec<_>>();

        diesel::insert_into(ticket_schema_form_fields::table)
            .values(records)
            .execute(conn)
            .await
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
    Insertable,
    Clone,
)]
#[diesel(belongs_to(TicketSchemaForm))]
#[diesel(table_name = ticket_schema_form_fields)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketSchemaFormField {
    pub id: i32,
    pub ticket_schema_form_id: i32,
    pub order: i32,
    pub key: String,
    pub description: String,
    pub define: FormFieldDefine<FormFieldOptionValue>,
    pub required: bool,
    pub editable: bool,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketSchemaFormField {
    pub async fn validate_and_normalize(
        &self,
        conn: &mut crate::DbConn,
        data: &serde_json::Value,
    ) -> Result<Value, String> {
        if self.editable {
            return Ok(serde_json::Value::Null);
        }

        fn is_same(left: &FormFieldOptionValue, right: &serde_json::Value) -> bool {
            match left {
                FormFieldOptionValue::String(lv) => {
                    if let serde_json::Value::String(rv) = right {
                        return lv == rv;
                    }
                }
                FormFieldOptionValue::Integer(lv) => {
                    if let Some(number) = right.as_i64() {
                        return *lv == number;
                    }
                }
            }
            false
        }

        match data {
            serde_json::Value::Null => {
                if self.required {
                    return Err(format!("Field {} is required", self.key));
                } else {
                    return Ok(data.clone());
                }
            }
            serde_json::Value::Bool(_) => {
                if let FormFieldDefine::Bool { .. } = self.define {
                    return Ok(data.clone());
                } else {
                    return Err(format!("Field {} is not a boolean", self.key));
                }
            }
            serde_json::Value::String(value) => match self.define {
                FormFieldDefine::SingleLineText { max_texts, .. } => {
                    let text = value.trim();
                    let text_len = text.len() as u32;
                    if text_len == 0 && self.required {
                        return Err(format!("Field {} is required", self.key));
                    }
                    if text_len > max_texts {
                        return Err(format!("Field {} is too long", self.key));
                    }
                    Ok(serde_json::Value::String(text.to_owned()))
                }
                FormFieldDefine::MultiLineText {
                    max_texts,
                    max_lines,
                    ..
                } => {
                    let text = value.trim();
                    let text_len = text.len() as u32;
                    if text_len == 0 && self.required {
                        return Err(format!("Field {} is required", self.key));
                    }
                    if text_len > max_texts {
                        return Err(format!("Field {} is too long", self.key));
                    }
                    if text.count_words("\n") > max_lines {
                        return Err(format!("Field {} is too many lines", self.key));
                    }
                    Ok(serde_json::Value::String(text.to_owned()))
                }
                FormFieldDefine::SingleChoice { ref options, .. } => {
                    if !options.iter().any(|o| is_same(&o.value, data)) {
                        return Err(format!("Field {} is not a valid choice", self.key));
                    }
                    return Ok(data.clone());
                }
                FormFieldDefine::File { .. } => {
                    if let Err(_) = TicketFormFile::find(conn, value.clone()).await {
                        return Err(format!("Field {} is not a upload file", self.key));
                    }
                    return Ok(data.clone());
                }
                FormFieldDefine::Image { .. } => {
                    if let Err(_) = TicketFormImage::find(conn, value.clone()).await {
                        return Err(format!("Field {} is not a upload image", self.key));
                    }
                    return Ok(data.clone());
                }
                _ => Err(format!("Field {} is not a string", self.key)),
            },
            serde_json::Value::Number(_) => match self.define {
                FormFieldDefine::SingleChoice { ref options, .. } => {
                    if !options.iter().any(|o| is_same(&o.value, data)) {
                        return Err(format!("Field {} is not a valid choice", self.key));
                    }
                    return Ok(data.clone());
                }
                _ => Err(format!("Field {} is not a number", self.key)),
            },
            serde_json::Value::Array(value) => {
                if let FormFieldDefine::MultipleChoice {
                    ref options,
                    max_options,
                    ..
                } = self.define
                {
                    let value_len = value.len() as u32;
                    if value_len == 0 && self.required {
                        return Err(format!("Field {} is required", self.key));
                    }
                    if value_len > max_options {
                        return Err(format!("Field {} is too many choices", self.key));
                    }
                    if !value
                        .iter()
                        .all(|v| options.iter().any(|o| is_same(&o.value, v)))
                    {
                        return Err(format!("Field {} is not a valid choice", self.key));
                    }
                    return Ok(data.clone());
                }
                Err(format!("Field {} is not an array", self.key))
            }
            _ => return Err(format!("Field {} is not a valid type", self.key)),
        }
    }

    pub async fn get_define_with_default_value(
        &self,
        conn: &mut crate::DbConn,
        user: &User,
    ) -> FormFieldDefine<FormFieldOptionValue> {
        match self.define.clone() {
            FormFieldDefine::SingleLineText { max_texts, default } => {
                let new_default = match default {
                    FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key,
                        ..
                    } => FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key: field_key.clone(),
                        value: TicketFormAnswer::get_field_value(
                            conn,
                            user,
                            &schema_form_id,
                            &flow_id,
                            &field_key,
                        )
                        .await,
                    },
                    FormFieldDefault::Static(_) => default.clone(),
                };
                FormFieldDefine::SingleLineText {
                    default: new_default,
                    max_texts,
                }
            }
            FormFieldDefine::MultiLineText {
                max_texts,
                max_lines,
                default,
            } => {
                let new_default = match default {
                    FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key,
                        ..
                    } => FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key: field_key.clone(),
                        value: TicketFormAnswer::get_field_value(
                            conn,
                            user,
                            &schema_form_id,
                            &flow_id,
                            &field_key,
                        )
                        .await,
                    },
                    FormFieldDefault::Static(_) => default.clone(),
                };
                FormFieldDefine::MultiLineText {
                    default: new_default,
                    max_texts,
                    max_lines,
                }
            }
            FormFieldDefine::SingleChoice {
                options, default, ..
            } => {
                let new_default = match default {
                    FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key,
                        ..
                    } => FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key: field_key.clone(),
                        value: TicketFormAnswer::get_field_value(
                            conn,
                            user,
                            &schema_form_id,
                            &flow_id,
                            &field_key,
                        )
                        .await,
                    },
                    FormFieldDefault::Static(_) => default.clone(),
                };
                FormFieldDefine::SingleChoice {
                    default: new_default,
                    options,
                }
            }
            FormFieldDefine::MultipleChoice {
                default,
                options,
                max_options,
                is_checkbox,
            } => {
                let new_default = match default {
                    FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key,
                        ..
                    } => FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key: field_key.clone(),
                        value: TicketFormAnswer::get_field_value(
                            conn,
                            user,
                            &schema_form_id,
                            &flow_id,
                            &field_key,
                        )
                        .await,
                    },
                    FormFieldDefault::Static(_) => default.clone(),
                };
                FormFieldDefine::MultipleChoice {
                    default: new_default,
                    options,
                    max_options,
                    is_checkbox,
                }
            }
            FormFieldDefine::Bool { default } => {
                let new_default = match default {
                    FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key,
                        ..
                    } => FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key: field_key.clone(),
                        value: TicketFormAnswer::get_field_value(
                            conn,
                            user,
                            &schema_form_id,
                            &flow_id,
                            &field_key,
                        )
                        .await,
                    },
                    FormFieldDefault::Static(_) => default.clone(),
                };
                FormFieldDefine::Bool {
                    default: new_default,
                }
            }
            FormFieldDefine::Image {
                default,
                max_size,
                min_width,
                max_width,
                min_height,
                max_height,
                mimes,
            } => {
                let new_default = match default {
                    FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key,
                        ..
                    } => FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key: field_key.clone(),
                        value: TicketFormAnswer::get_field_value(
                            conn,
                            user,
                            &schema_form_id,
                            &flow_id,
                            &field_key,
                        )
                        .await,
                    },
                    FormFieldDefault::Static(_) => default.clone(),
                };
                FormFieldDefine::Image {
                    default: new_default,
                    max_size,
                    min_width,
                    max_width,
                    min_height,
                    max_height,
                    mimes,
                }
            }
            FormFieldDefine::File {
                default,
                max_size,
                mimes,
            } => {
                let new_default = match default {
                    FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key,
                        ..
                    } => FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key: field_key.clone(),
                        value: TicketFormAnswer::get_field_value(
                            conn,
                            user,
                            &schema_form_id,
                            &flow_id,
                            &field_key,
                        )
                        .await,
                    },
                    FormFieldDefault::Static(_) => default.clone(),
                };
                FormFieldDefine::File {
                    default: new_default,
                    max_size,
                    mimes,
                }
            }
            FormFieldDefine::IfEqual { key, from, value } => {
                let new_from = match from {
                    FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key,
                        ..
                    } => FormFieldDefault::Dynamic {
                        schema_form_id,
                        flow_id,
                        field_key: field_key.clone(),
                        value: TicketFormAnswer::get_field_value(
                            conn,
                            user,
                            &schema_form_id,
                            &flow_id,
                            &field_key,
                        )
                        .await,
                    },
                    FormFieldDefault::Static(_) => from.clone(),
                };
                FormFieldDefine::IfEqual {
                    key,
                    from: new_from,
                    value,
                }
            }
            _ => self.define.clone(),
        }
    }

    pub async fn get_results(
        &self,
        conn: &mut crate::DbConn,
        data: &serde_json::Value,
    ) -> serde_json::Value {
        if self.is_file_define() {
            if let Ok(file) = TicketFormFile::find(conn, data.as_str().unwrap().to_owned()).await {
                return serde_json::json!({
                    "id": file.id,
                    "path": file.path,
                    "mime": file.mime,
                    "size": file.size
                });
            }
        } else if self.is_image_define() {
            if let Ok(image) = TicketFormImage::find(conn, data.as_str().unwrap().to_owned()).await
            {
                return serde_json::json!({
                    "id": image.id,
                    "path": image.path,
                    "mime": image.mime,
                    "size": image.size,
                    "width": image.width,
                    "height": image.height
                });
            }
        }
        data.clone()
    }
}

impl TicketSchemaFormField {
    pub async fn find(
        conn: &mut crate::DbConn,
        id: i32,
    ) -> Result<TicketSchemaFormField, diesel::result::Error> {
        ticket_schema_form_fields::table.find(id).first(conn).await
    }

    pub async fn save(&self, conn: &mut crate::DbConn) -> Result<usize, diesel::result::Error> {
        diesel::replace_into(ticket_schema_form_fields::table)
            .values(self)
            .execute(conn)
            .await
    }

    pub fn is_file_define(&self) -> bool {
        if let FormFieldDefine::File { .. } = self.define {
            return true;
        }
        false
    }

    pub fn is_image_define(&self) -> bool {
        if let FormFieldDefine::Image { .. } = self.define {
            return true;
        }
        false
    }

    pub async fn delete(&self, conn: &mut crate::DbConn) -> Result<usize, diesel::result::Error> {
        diesel::delete(
            ticket_schema_form_fields::table.filter(ticket_schema_form_fields::id.eq(self.id)),
        )
        .execute(conn)
        .await
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
    Insertable,
)]
#[diesel(belongs_to(TicketFlow))]
#[diesel(belongs_to(TicketSchemaForm))]
#[diesel(table_name = ticket_form_answers)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketFormAnswer {
    pub id: i32,
    pub ticket_flow_id: i32,
    pub ticket_schema_form_id: i32,
    pub value: serde_json::Value,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketFormAnswer {
    pub async fn create(
        conn: &mut crate::DbConn,
        flow: &TicketFlow,
        form_schema: &FormSchema,
        value: serde_json::Map<String, Value>,
    ) -> Result<TicketFormAnswer, diesel::result::Error> {
        let FormSchema { form, .. } = form_schema;

        diesel::insert_into(ticket_form_answers::table)
            .values((
                ticket_form_answers::ticket_flow_id.eq(flow.id),
                ticket_form_answers::ticket_schema_form_id.eq(form.id),
                ticket_form_answers::value.eq(serde_json::Value::Object(value)),
            ))
            .execute(conn)
            .await?;

        sql_function! {
            fn last_insert_id() -> Integer;
        }

        ticket_form_answers::table
            .find(last_insert_id())
            .first(conn)
            .await
    }

    pub async fn get_field_value(
        conn: &mut crate::DbConn,
        user: &User,
        ticket_schema_form_id: &i32,
        flow_id: &Option<i32>,
        field_key: &String,
    ) -> Option<FormFieldValue> {
        let mut query = ticket_form_answers::table
            .inner_join(ticket_flows::table)
            .filter(ticket_flows::user_id.eq(user.id.clone()))
            .filter(ticket_form_answers::ticket_schema_form_id.eq(ticket_schema_form_id))
            .into_boxed();

        if let Some(id) = flow_id {
            query = query.filter(ticket_flows::id.eq(id));
        }

        let answer: Result<TicketFormAnswer, _> = query
            .order(ticket_form_answers::id.desc())
            .select(TicketFormAnswer::as_select())
            .first::<TicketFormAnswer>(conn)
            .await;

        match answer {
            Ok(answer) => match answer.value.get(field_key) {
                Some(value) => match serde_json::from_value::<FormFieldValue>(value.clone()) {
                    Ok(value) => Some(value),
                    Err(_) => None,
                },
                None => None,
            },
            Err(_) => None,
        }
    }

    pub async fn save(&self, conn: &mut crate::DbConn) -> Result<usize, diesel::result::Error> {
        diesel::replace_into(ticket_form_answers::table)
            .values(self)
            .execute(conn)
            .await
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
    Insertable,
)]
#[diesel(belongs_to(TicketSchemaFormField))]
#[diesel(table_name = ticket_form_images)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketFormImage {
    pub id: String,
    pub ticket_schema_form_field_id: i32,
    pub path: String,
    pub mime: ImageMime,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketFormImage {
    pub async fn create(
        conn: &mut crate::DbConn,
        id: String,
        field: &TicketSchemaFormField,
        path: String,
        mime: ImageMime,
        size: u32,
        width: u32,
        height: u32,
    ) -> Result<TicketFormImage, diesel::result::Error> {
        diesel::insert_into(ticket_form_images::table)
            .values((
                ticket_form_images::id.eq(id),
                ticket_form_images::ticket_schema_form_field_id.eq(field.id),
                ticket_form_images::path.eq(path),
                ticket_form_images::mime.eq(mime),
                ticket_form_images::size.eq(size),
                ticket_form_images::width.eq(width),
                ticket_form_images::height.eq(height),
            ))
            .execute(conn)
            .await?;

        sql_function! {
            fn last_insert_id() -> VarChar;
        }

        ticket_form_images::table
            .find(last_insert_id())
            .first(conn)
            .await
    }
    pub async fn find(
        conn: &mut crate::DbConn,
        id: String,
    ) -> Result<TicketFormImage, diesel::result::Error> {
        ticket_form_images::table.find(id).first(conn).await
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
    Insertable,
)]
#[diesel(belongs_to(TicketSchemaFormField))]
#[diesel(table_name = ticket_form_files)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct TicketFormFile {
    pub id: String,
    pub ticket_schema_form_field_id: i32,
    pub path: String,
    pub mime: FileMime,
    pub size: u32,
    #[serde(with = "unix_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "unix_time")]
    pub updated_at: NaiveDateTime,
}

impl TicketFormFile {
    pub async fn create(
        conn: &mut crate::DbConn,
        id: String,
        field: &TicketSchemaFormField,
        path: String,
        mime: FileMime,
        size: u32,
    ) -> Result<TicketFormFile, diesel::result::Error> {
        diesel::insert_into(ticket_form_files::table)
            .values((
                ticket_form_files::id.eq(id),
                ticket_form_files::ticket_schema_form_field_id.eq(field.id),
                ticket_form_files::path.eq(path),
                ticket_form_files::mime.eq(mime),
                ticket_form_files::size.eq(size),
            ))
            .execute(conn)
            .await?;

        sql_function! {
            fn last_insert_id() -> VarChar;
        }

        ticket_form_files::table
            .find(last_insert_id())
            .first(conn)
            .await
    }
    pub async fn find(
        conn: &mut crate::DbConn,
        id: String,
    ) -> Result<TicketFormFile, diesel::result::Error> {
        ticket_form_files::table.find(id).first(conn).await
    }
}
