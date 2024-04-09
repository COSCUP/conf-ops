use rocket::fs::TempFile;
use rocket::{serde::json::serde_json, State};
use serde_json::Value;
use sha256::digest;
use tokio::io::AsyncReadExt;

use crate::{
    utils::{
        file::FileMime,
        i18n::I18n,
        image::{self, ImageMime},
    },
    DataFolder,
};

use self::{
    fields::{FormFieldDefault, FormFieldDefine},
    models::{TicketFormFile, TicketFormImage, TicketSchemaForm, TicketSchemaFormField},
};

pub mod fields;
pub mod models;

#[derive(Serialize, Deserialize, Debug)]
pub struct FormSchema {
    pub form: TicketSchemaForm,
    pub fields: Vec<TicketSchemaFormField>,
}

impl FormSchema {
    pub async fn new_with_ticket(
        conn: &mut crate::DbConn,
        ticket_id: &i32,
        form: TicketSchemaForm,
        fields: Vec<TicketSchemaFormField>,
    ) -> FormSchema {
        let mut new_fields = FormSchema::get_processed_fields(conn, fields, ticket_id).await;
        new_fields.sort_by_key(|field| field.order);
        FormSchema {
            form,
            fields: new_fields,
        }
    }

    pub fn new(form: TicketSchemaForm, fields: Vec<TicketSchemaFormField>) -> FormSchema {
        let mut new_fields = fields.clone();
        new_fields.sort_by_key(|field| field.order);
        FormSchema {
            form,
            fields: new_fields,
        }
    }

    pub async fn validate_and_normalize<'a>(
        &self,
        conn: &mut crate::DbConn,
        i18n: &I18n<'a>,
        data: &serde_json::Map<String, Value>
    ) -> Result<serde_json::Map<String, Value>, serde_json::Map<String, Value>> {
        let FormSchema { fields, .. } = self;

        let mut is_error = false;
        let mut result = serde_json::Map::new();
        let mut errors = serde_json::Map::new();

        let mut last_falsy_if: Option<String> = None;
        for field in fields.iter() {
            if let Some(ref falsy_if_key) = last_falsy_if {
                if let TicketSchemaFormField {
                    define: FormFieldDefine::IfEnd { key },
                    ..
                } = field
                {
                    if key == falsy_if_key {
                        last_falsy_if = None;
                    }
                }
                continue;
            }
            if let TicketSchemaFormField {
                define: FormFieldDefine::IfEqual { key, from, value },
                ..
            } = field.clone()
            {
                let condition_result = match from {
                    FormFieldDefault::Static(from_value) => value.contains(&from_value),
                    FormFieldDefault::Dynamic {
                        value: from_value, ..
                    } => match from_value {
                        Some(from_value) => value.contains(&from_value),
                        None => false,
                    },
                };
                if !condition_result {
                    last_falsy_if = Some(key);
                }
                continue;
            }
            let user_value = match data.get::<String>(&field.key) {
                Some(value) => value,
                None => {
                    if field.required {
                        is_error = true;
                        errors.insert(
                            field.key.clone(),
                            serde_json::Value::String(format!("Field {} is required", field.key)),
                        );
                    }
                    continue;
                }
            };
            let new_value = match field.validate_and_normalize(conn, i18n, user_value).await {
                Ok(value) => value,
                Err(err) => {
                    is_error = true;
                    errors.insert(field.key.clone(), serde_json::Value::String(err));
                    continue;
                }
            };
            if let serde_json::Value::Null = new_value {
                continue;
            }
            result.insert(field.key.clone(), new_value);
        }

        if is_error {
            Err(errors)
        } else {
            Ok(result)
        }
    }

    pub async fn get_processed_fields(
        conn: &mut crate::DbConn,
        fields: Vec<TicketSchemaFormField>,
        ticket_id: &i32,
    ) -> Vec<TicketSchemaFormField> {
        let mut result: Vec<TicketSchemaFormField> = vec![];
        let mut last_falsy_if: Option<String> = None;
        for raw_field in fields.iter() {
            if let Some(ref falsy_if_key) = last_falsy_if {
                if let TicketSchemaFormField {
                    define: FormFieldDefine::IfEnd { key },
                    ..
                } = raw_field
                {
                    if key == falsy_if_key {
                        last_falsy_if = None;
                    }
                }
                continue;
            }
            let mut field = raw_field.clone();
            let new_define = field.get_define_with_default_value(conn, ticket_id).await;
            field.define = new_define;
            if let TicketSchemaFormField {
                define: FormFieldDefine::IfEqual { key, from, value },
                ..
            } = field.clone()
            {
                let condition_result = match from {
                    FormFieldDefault::Static(from_value) => value.contains(&from_value),
                    FormFieldDefault::Dynamic {
                        value: from_value, ..
                    } => match from_value {
                        Some(from_value) => value.contains(&from_value),
                        None => false,
                    },
                };
                if !condition_result {
                    last_falsy_if = Some(key);
                }
                continue;
            }
            if let TicketSchemaFormField {
                define: FormFieldDefine::IfEnd { .. },
                ..
            } = raw_field
            {
                continue;
            }
            result.push(field);
        }
        result
    }

    pub async fn get_results(
        &self,
        conn: &mut crate::DbConn,
        data: &serde_json::Map<String, Value>,
    ) -> serde_json::Map<String, Value> {
        let mut result = serde_json::Map::new();
        for (key, value) in data.iter() {
            let field = self.fields.iter().find(|field| field.key == *key).unwrap();
            let new_value = field.get_results(conn, value).await;
            result.insert(key.clone(), new_value);
        }
        result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PartFormSchema {
    pub form: TicketSchemaForm,
    pub field: TicketSchemaFormField,
}

impl PartFormSchema {
    pub async fn upload_image<'a>(
        &self,
        conn: &mut crate::DbConn,
        data_folder: &State<DataFolder>,
        i18n: &I18n<'a>,
        mut temp_file: TempFile<'_>,
    ) -> Result<TicketFormImage, String> {
        let PartFormSchema { field, .. } = self;

        let get_file_content = || async {
            let mut file = temp_file
                .open()
                .await
                .map_err(|err| format!("Error opening file: {}", err))?;
            let mut file_content = vec![];
            file.read_to_end(&mut file_content)
                .await
                .map_err(|err| format!("Error reading file: {}", err))?;

            Ok::<_, String>(file_content)
        };

        let file_size = temp_file.len() as u32;

        if let FormFieldDefine::Image {
            max_size,
            min_width,
            max_width,
            min_height,
            max_height,
            ref mimes,
            ..
        } = field.define
        {
            if file_size > max_size {
                return Err(i18n.tf("ticket.rules.image_too_large", &[("field", field.key.clone())]));
            }

            let file_content = get_file_content().await?;

            let handle_raster_image = || async {
                let (mime, (width, height), _) = image::get_raster_image(&file_content)
                    .map_err(|err| format!("Error getting image: {}", err))?;

                if let Some(min_width) = min_width {
                    if width < min_width {
                        return Err(i18n.tf("ticket.rules.image_width_too_small", &[("field", field.key.clone())]));
                    }
                }

                if let Some(max_width) = max_width {
                    if width > max_width {
                        return Err(i18n.tf("ticket.rules.image_width_too_large", &[("field", field.key.clone())]));
                    }
                }

                if let Some(min_height) = min_height {
                    if height < min_height {
                        return Err(i18n.tf("ticket.rules.image_height_too_small", &[("field", field.key.clone())]));
                    }
                }

                if let Some(max_height) = max_height {
                    if height > max_height {
                        return Err(i18n.tf("ticket.rules.image_height_too_large", &[("field", field.key.clone())]));
                    }
                }

                match mime {
                    Some(mime) => {
                        if !mimes.contains(&mime) {
                            return Err(i18n.tf("ticket.rules.invalid_image_type", &[("field", field.key.clone())]));
                        }
                        Ok((mime, (width, height), file_size))
                    }
                    _ => return Err(i18n.tf("ticket.rules.invalid_image_type", &[("field", field.key.clone())])),
                }
            };

            let (mime, (width, height), size) = match temp_file.content_type() {
                Some(content_type) => {
                    if content_type.is_svg() {
                        if !mimes.contains(&ImageMime::Svg) {
                            return Err(format!(
                                "Image {} is not in the correct format",
                                field.key
                            ));
                        }
                        (ImageMime::Svg, (0 as u32, 0 as u32), file_size)
                    } else {
                        handle_raster_image().await?
                    }
                }
                _ => handle_raster_image().await?,
            };

            let file_extension = &temp_file
                .content_type()
                .and_then(|f| f.extension())
                .ok_or(format!("Error getting file extension"))?;
            let hash = digest(&file_content);
            let file_name = format!("{}.{}", hash, file_extension);
            let file_path = data_folder.image_path(&file_name);
            temp_file
                .move_copy_to(&file_path)
                .await
                .map_err(|err| format!("Error saving file: {}", err))?;

            let path = file_path
                .to_str()
                .ok_or(format!("Error getting file path"))?;

            let image = TicketFormImage {
                id: hash.clone(),
                ticket_schema_form_field_id: field.id,
                path: path.to_owned(),
                mime: mime.clone(),
                size,
                width,
                height,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            };

            image
                .save(conn)
                .await
                .map_err(|err| format!("Error saving image: {}", err))?;

            return Ok(image);
        };

        Err(format!("Field {} is not an image", field.key))
    }

    pub async fn upload_file<'a>(
        &self,
        conn: &mut crate::DbConn,
        data_folder: &State<DataFolder>,
        i18n: &I18n<'a>,
        mut temp_file: TempFile<'_>,
    ) -> Result<TicketFormFile, String> {
        let PartFormSchema { field, .. } = self;

        let get_file_content = || async {
            let mut file = temp_file
                .open()
                .await
                .map_err(|err| format!("Error opening file: {}", err))?;
            let mut file_content = vec![];
            file.read_to_end(&mut file_content)
                .await
                .map_err(|err| format!("Error reading file: {}", err))?;

            Ok::<_, String>(file_content)
        };

        let file_size = temp_file.len() as u32;

        if let FormFieldDefine::File {
            max_size,
            ref mimes,
            ..
        } = field.define
        {
            if file_size > max_size {
                return Err(i18n.tf("ticket.rules.file_too_large", &[("field", field.key.clone())]));
            }

            let file_content = get_file_content().await?;

            let mime = match temp_file.content_type() {
                Some(content_type) => {
                    let mime = FileMime::from_content_type(content_type);
                    match mime {
                        Some(mime) => {
                            if !mimes.contains(&mime) {
                                return Err(i18n.tf("ticket.rules.invalid_file_type", &[("field", field.key.clone())]));
                            }
                            mime
                        }
                        _ => {
                            return Err(i18n.tf("ticket.rules.invalid_file_type", &[("field", field.key.clone())]))
                        }
                    }
                }
                _ => return Err(i18n.tf("ticket.rules.invalid_file_type", &[("field", field.key.clone())])),
            };

            let file_extension = &temp_file
                .content_type()
                .and_then(|f| f.extension())
                .ok_or(format!("Error getting file extension"))?;
            let hash = digest(&file_content);
            let file_name = format!("{}.{}", hash, file_extension);
            let file_path = data_folder.file_path(&file_name);
            temp_file
                .move_copy_to(&file_path)
                .await
                .map_err(|err| format!("Error saving file: {}", err))?;

            let path = file_path
                .to_str()
                .ok_or(format!("Error getting file path"))?;

            let file = TicketFormFile::create(conn, hash, field, path.to_owned(), mime, file_size)
                .await
                .map_err(|err| format!("Error saving file: {}", err))?;
            return Ok(file);
        }

        Err(format!("Field {} is not a file", field.key))
    }
}
