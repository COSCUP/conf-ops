use std::fmt::Debug;

use diesel::{
    deserialize::FromSql,
    mysql::{Mysql, MysqlValue},
    serialize::ToSql,
    sql_types,
};
use rocket::serde::json::serde_json;

use crate::utils::{file::FileMime, image::ImageMime};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "type", content = "content")]
pub enum FormFieldDefault {
    Static(FormFieldValue),
    Dynamic {
        schema_form_id: i32,
        flow_id: Option<i32>,
        field_key: String,
        value: Option<FormFieldValue>,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct FormFieldOption<T> {
    pub text: String,
    pub value: T,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum FormFieldOptionValue {
    Integer(i64),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum FormFieldValue {
    Integer(i32),
    String(String),
    Bool(bool),
    Array(Vec<FormFieldOptionValue>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FormTextFieldType {
    String,
    Email,
    Url,
}

#[derive(FromSqlRow, AsExpression, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "type")]
#[diesel(sql_type = sql_types::Json)]
pub enum FormFieldDefine<OV> {
    SingleLineText {
        text_type: Option<FormTextFieldType>,
        max_texts: u32,
        default: Option<FormFieldDefault>,
    },
    MultiLineText {
        max_texts: u32,
        max_lines: u32,
        default: Option<FormFieldDefault>,
    },
    SingleChoice {
        options: Vec<FormFieldOption<OV>>,
        default: Option<FormFieldDefault>,
    },
    MultipleChoice {
        options: Vec<FormFieldOption<OV>>,
        max_options: u32,
        is_checkbox: bool,
        default: Option<FormFieldDefault>,
    },
    Bool {
        default: Option<FormFieldDefault>,
    },
    Image {
        max_size: u32,
        min_width: Option<u32>,
        max_width: Option<u32>,
        min_height: Option<u32>,
        max_height: Option<u32>,
        mimes: Vec<ImageMime>,
        default: Option<FormFieldDefault>,
    },
    File {
        max_size: u32,
        mimes: Vec<FileMime>,
        default: Option<FormFieldDefault>,
    },
    IfEqual {
        key: String,
        from: FormFieldDefault,
        value: Vec<FormFieldValue>,
    },
    IfEnd {
        key: String,
    },
}

impl FromSql<sql_types::Json, Mysql> for FormFieldDefine<FormFieldOptionValue> {
    fn from_sql(bytes: MysqlValue) -> diesel::deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<sql_types::Json, Mysql>>::from_sql(bytes)?;
        Ok(serde_json::from_value::<
            FormFieldDefine<FormFieldOptionValue>,
        >(value)?)
    }
}

impl ToSql<sql_types::Json, Mysql> for FormFieldDefine<FormFieldOptionValue> {
    fn to_sql(&self, out: &mut diesel::serialize::Output<Mysql>) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;
        <serde_json::Value as ToSql<sql_types::Json, Mysql>>::to_sql(&value, &mut out.reborrow())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct FormSchemaField {
    pub name_zh: String,
    pub name_en: String,
    pub description_zh: String,
    pub description_en: String,
    pub key: String,
    pub define: FormFieldDefine<FormFieldOptionValue>,
    pub required: bool,
    pub editable: bool,
}
