use rocket::fs::TempFile;
use rocket::{serde::json::serde_json, State};
use serde_json::Value;
use sha256::digest;
use tokio::io::AsyncReadExt;

use crate::{
    models::user::User,
    utils::{
        file::FileMime,
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
    pub async fn new_with_user(
        conn: &mut crate::DbConn,
        user: &User,
        form: TicketSchemaForm,
        fields: Vec<TicketSchemaFormField>,
    ) -> FormSchema {
        let mut new_fields = FormSchema::get_processed_fields(conn, fields, user).await;
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

    pub async fn validate_and_normalize(
        &self,
        conn: &mut crate::DbConn,
        data: &serde_json::Map<String, Value>,
    ) -> Result<serde_json::Map<String, Value>, serde_json::Map<String, Value>> {
        let FormSchema { fields, .. } = self;

        let mut is_error = false;
        let mut result = serde_json::Map::new();
        let mut errors = serde_json::Map::new();
        for field in fields.iter() {
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
            let new_value = match field.validate_and_normalize(conn, user_value).await {
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
        user: &User,
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
            let new_define = field.get_define_with_default_value(conn, user).await;
            field.define = new_define;
            if let TicketSchemaFormField {
                define: FormFieldDefine::IfEqual { key, from, value },
                ..
            } = field.clone()
            {
                let condition_result = match from {
                    FormFieldDefault::Static(from_value) => from_value == value,
                    FormFieldDefault::Dynamic {
                        value: from_value, ..
                    } => match from_value {
                        Some(from_value) => from_value == value,
                        None => false,
                    },
                };
                if !condition_result {
                    last_falsy_if = Some(key);
                    continue;
                }
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
    pub async fn upload_image(
        &self,
        conn: &mut crate::DbConn,
        data_folder: &State<DataFolder>,
        mut temp_file: TempFile<'_>,
    ) -> Result<serde_json::Value, String> {
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
                return Err(format!("Image {} is too large", field.key));
            }

            let file_content = get_file_content().await?;

            let handle_raster_image = || async {
                let (mime, (width, height), _) = image::get_raster_image(&file_content)
                    .map_err(|err| format!("Error getting image: {}", err))?;

                if let Some(min_width) = min_width {
                    if width < min_width {
                        return Err(format!("Image {} width is too small", field.key));
                    }
                }

                if let Some(max_width) = max_width {
                    if width > max_width {
                        return Err(format!("Image {} width is too large", field.key));
                    }
                }

                if let Some(min_height) = min_height {
                    if height < min_height {
                        return Err(format!("Image {} height is too small", field.key));
                    }
                }

                if let Some(max_height) = max_height {
                    if height > max_height {
                        return Err(format!("Image {} height is too large", field.key));
                    }
                }

                match mime {
                    Some(mime) => {
                        if !mimes.contains(&mime) {
                            return Err(format!(
                                "Image {} is not in the correct format",
                                field.key
                            ));
                        }
                        Ok((mime, (width, height), file_size))
                    }
                    _ => return Err(format!("Image {} is not in the correct format", field.key)),
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
                .persist_to(&file_path)
                .await
                .map_err(|err| format!("Error saving file: {}", err))?;

            let path = file_path
                .to_str()
                .ok_or(format!("Error getting file path"))?;

            let image = TicketFormImage::create(
                conn,
                hash,
                field,
                path.to_owned(),
                mime,
                size,
                width,
                height,
            )
            .await
            .map_err(|err| format!("Error saving image: {}", err))?;
            return Ok(Value::String(image.id));
        };

        Err(format!("Field {} is not an image", field.key))
    }

    pub async fn upload_file(
        &self,
        conn: &mut crate::DbConn,
        data_folder: &State<DataFolder>,
        mut temp_file: TempFile<'_>,
    ) -> Result<serde_json::Value, String> {
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
                return Err(format!("File {} is too large", field.key));
            }

            let file_content = get_file_content().await?;

            let mime = match temp_file.content_type() {
                Some(content_type) => {
                    let mime = FileMime::from_content_type(content_type);
                    match mime {
                        Some(mime) => {
                            if !mimes.contains(&mime) {
                                return Err(format!(
                                    "File {} is not in the correct format",
                                    field.key
                                ));
                            }
                            mime
                        }
                        _ => {
                            return Err(format!("File {} is not in the correct format", field.key))
                        }
                    }
                }
                _ => return Err(format!("File {} is not in the correct format", field.key)),
            };

            let file_extension = &temp_file
                .content_type()
                .and_then(|f| f.extension())
                .ok_or(format!("Error getting file extension"))?;
            let hash = digest(&file_content);
            let file_name = format!("{}.{}", hash, file_extension);
            let file_path = data_folder.file_path(&file_name);
            temp_file
                .persist_to(&file_path)
                .await
                .map_err(|err| format!("Error saving file: {}", err))?;

            let path = file_path
                .to_str()
                .ok_or(format!("Error getting file path"))?;

            let file = TicketFormFile::create(conn, hash, field, path.to_owned(), mime, file_size)
                .await
                .map_err(|err| format!("Error saving file: {}", err))?;
            return Ok(Value::String(file.id));
        }

        Err(format!("Field {} is not a file", field.key))
    }
}
