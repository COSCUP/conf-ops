use diesel::{
    deserialize::FromSql,
    mysql::{Mysql, MysqlValue},
    serialize::ToSql,
    sql_types,
};
use rocket::http::ContentType;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(FromSqlRow, AsExpression, Debug, PartialEq, Clone)]
#[diesel(sql_type = sql_types::VarChar)]
pub enum FileMime {
    Pdf,
    Doc,
    Docx,
    Xls,
    Xlsx,
    Ppt,
    Pptx,
    Odt,
    Ods,
    Odp,
    Txt,
    Csv,
}

impl FileMime {
    pub fn as_str(&self) -> &str {
        match self {
            FileMime::Pdf => "application/pdf",
            FileMime::Doc => "application/msword",
            FileMime::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            FileMime::Xls => "application/vnd.ms-excel",
            FileMime::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            FileMime::Ppt => "application/vnd.ms-powerpoint",
            FileMime::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            FileMime::Odt => "application/vnd.oasis.opendocument.text",
            FileMime::Ods => "application/vnd.oasis.opendocument.spreadsheet",
            FileMime::Odp => "application/vnd.oasis.opendocument.presentation",
            FileMime::Txt => "text/plain",
            FileMime::Csv => "text/csv",
        }
    }
    pub fn from_content_type(content_type: &ContentType) -> Option<Self> {
        let top = content_type.top().as_str().to_lowercase();
        let sub = content_type.sub().as_str().to_lowercase();
        let mime = format!("{}/{}", top, sub);
        Self::from_str(&mime)
    }
    pub fn from_str(s: &str) -> Option<FileMime> {
        match s {
            "application/pdf" => Some(FileMime::Pdf),
            "application/msword" => Some(FileMime::Doc),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                Some(FileMime::Docx)
            }
            "application/vnd.ms-excel" => Some(FileMime::Xls),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                Some(FileMime::Xlsx)
            }
            "application/vnd.ms-powerpoint" => Some(FileMime::Ppt),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
                Some(FileMime::Pptx)
            }
            "application/vnd.oasis.opendocument.text" => Some(FileMime::Odt),
            "application/vnd.oasis.opendocument.spreadsheet" => Some(FileMime::Ods),
            "application/vnd.oasis.opendocument.presentation" => Some(FileMime::Odp),
            "text/plain" => Some(FileMime::Txt),
            "text/csv" => Some(FileMime::Csv),
            _ => None,
        }
    }
}

impl Serialize for FileMime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FileMime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FileMime::from_str(&s).ok_or_else(|| serde::de::Error::custom("Invalid file format"))
    }
}

impl FromSql<sql_types::VarChar, Mysql> for FileMime {
    fn from_sql(value: MysqlValue) -> diesel::deserialize::Result<Self> {
        let s = <String as FromSql<sql_types::VarChar, Mysql>>::from_sql(value)?;
        FileMime::from_str(&s).ok_or_else(|| "Invalid file format".into())
    }
}

impl ToSql<sql_types::VarChar, Mysql> for FileMime {
    fn to_sql<'a>(
        &'a self,
        out: &mut diesel::serialize::Output<'a, '_, Mysql>,
    ) -> diesel::serialize::Result {
        ToSql::<sql_types::VarChar, Mysql>::to_sql(self.as_str(), out)
    }
}
